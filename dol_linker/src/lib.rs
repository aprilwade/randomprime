
use enum_map::{Enum, EnumMap};
use goblin::elf::{self, Elf};
use goblin::Object;

use memmap::{Mmap, MmapOptions};

use scroll::{ctx, IOwrite, Cwrite, SizeWith};
use snafu::{ensure, OptionExt, ResultExt, Snafu};


use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom};
use std::iter;
use std::path::{Path, PathBuf};
use std::rc::Rc;


// XXX This is a throughly awful hack
static ZEROES: &[u8] = &[0; 4096];

#[derive(Debug, Snafu)]
pub enum Error
{
    #[snafu(display("Could not open file {}: {}", filename.display(), source))]
    OpenFile {
        filename: PathBuf,
        source: std::io::Error
    },
    #[snafu(display("Could not write to file {}: {}", filename.display(), source))]
    WriteFile {
        filename: PathBuf,
        source: std::io::Error
    },
    #[snafu(display("Failed parsing object file {}: {}", filename.display(), source))]
    ObjectParsing {
        filename: PathBuf,
        source: goblin::error::Error,
    },
    #[snafu(display("Unrecognized or unknown object file format {}", filename.display()))]
    ObjectFormat {
        filename: PathBuf,
    },
    #[snafu(display("Failed parsing symbol table file {} on line {}: {}",
                    filename.display(), line_number, source))]
    SymTableAddrParsing {
        filename: PathBuf,
        line_number: usize,
        source: std::num::ParseIntError,
    },
    #[snafu(display("Failed parsing symbol table file {} on line {}: {}",
                    filename.display(), line_number, source))]
    SymTableIO {
        filename: PathBuf,
        line_number: usize,
        source: std::io::Error,
    },
    #[snafu(display("Failed parsing symbol table file {} on line {}: Duplicate entry",
                    filename.display(), line_number))]
    SymTableDuplicateEntry {
        filename: PathBuf,
        line_number: usize,
    },
    #[snafu(display("Failed parsing symbol table file {} on line {}: Wrong number of componenets",
                    filename.display(), line_number))]
    SymTableWrongNumberOfComponenets {
        filename: PathBuf,
        line_number: usize,
    },

    #[snafu(display("Unresolved symbol: {}", symbol_name))]
    UnresolvedSymbol {
        symbol_name: String,
    },
    #[snafu(display("Duplicate symbol: {}", symbol_name))]
    DuplicateSymbol {
        symbol_name: String,
    },
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, IOwrite, SizeWith)]
struct RelHeader
{
    module_id: u32,

    next_module_link: u32,
    prev_module_link: u32,

    section_count: u32,
    section_table_offset: u32,


    module_name_offset: u32,
    module_name_size: u32,

    version: u32,

    bss_size: u32,

    reloc_table_offset: u32,
    import_table_offset: u32,
    import_table_size: u32,

    prolog_function_section: u8,
    epilog_function_section: u8,
    unresolved_function_section: u8,

    padding: u8,

    prolog_function_offset: u32,
    epilog_function_offset: u32,
    unresolved_function_offset: u32,
}

#[derive(Debug, Clone, Copy)]
struct RelSectionInfo
{
    size: u32,
    is_executable: bool,
    offset: u32,
}

impl<C> ctx::SizeWith<C> for RelSectionInfo
{
    fn size_with(_: &C) -> usize
    {
        8
    }
}

impl<C: Copy> ctx::IntoCtx<C> for RelSectionInfo
{
    fn into_ctx(self, w: &mut [u8], _: C)
    {

        let real_offset = self.offset | if self.is_executable { 1 } else { 0 };
        w.cwrite_with(real_offset, 0, scroll::BE);
        w.cwrite_with(self.size, 4, scroll::BE);
    }
}


#[derive(Debug, Clone, Copy, IOwrite, SizeWith)]
struct RelImport
{
    module_id: u32,
    relocations_offset: u32,
}


#[derive(Debug, Clone, Copy, IOwrite, SizeWith)]
struct RelRelocation
{
    offset: u16,
    relocation_type: u8,
    section_index: u8,
    symbol_offset: u32,
}


impl RelRelocation
{
    fn start_section_entry(section_index: u8) -> RelRelocation
    {
        RelRelocation {
            offset: 0,
            relocation_type: RelRelocationType::R_DOLPHIN_SECTION as u8,
            section_index,
            symbol_offset: 0,
        }
    }

    fn end_relocations_entry() -> RelRelocation
    {
        RelRelocation {
            offset: 0,
            relocation_type: RelRelocationType::R_DOLPHIN_END as u8,
            section_index: 0,
            symbol_offset: 0,
        }
    }
}


macro_rules! build_ppc_reloc_types {
    ($enum_name:ident { $($name:ident : $value:expr,)* }) => {
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        #[repr(u8)]
        enum $enum_name
        {
            $($name = $value,)*
        }

        impl $enum_name
        {
            fn from_u32(i: u32) -> Option<$enum_name>
            {
                match i {
                    $($value => Some($enum_name::$name),)*
                    _ => None,
                }
            }
        }
    };
}

build_ppc_reloc_types! {
    ElfRelocationType
    {
        R_PPC_NONE: 0,
        R_PPC_ADDR32: 1,
        R_PPC_ADDR24: 2,
        R_PPC_ADDR16: 3,
        R_PPC_ADDR16_LO: 4,
        R_PPC_ADDR16_HI: 5,
        R_PPC_ADDR16_HA: 6,
        R_PPC_ADDR14: 7,
        R_PPC_ADDR14_BRTAKEN: 8,
        R_PPC_ADDR14_BRNTAKEN: 9,
        R_PPC_REL24: 10,
        R_PPC_REL14: 11,
        R_PPC_REL14_BRTAKEN: 12,
        R_PPC_REL14_BRNTAKEN: 13,
        R_PPC_GOT16: 14,
        R_PPC_GOT16_LO: 15,
        R_PPC_GOT16_HI: 16,
        R_PPC_GOT16_HA: 17,
        R_PPC_PLTREL24: 18,
        R_PPC_COPY: 19,
        R_PPC_GLOB_DAT: 20,
        R_PPC_JMP_SLOT: 21,
        R_PPC_RELATIVE: 22,
        R_PPC_LOCAL24PC: 23,
        R_PPC_UADDR32: 24,
        R_PPC_UADDR16: 25,
        R_PPC_REL32: 26,
        R_PPC_PLT32: 27,
        R_PPC_PLTREL32: 28,
        R_PPC_PLT16_LO: 29,
        R_PPC_PLT16_HI: 30,
        R_PPC_PLT16_HA: 31,
        R_PPC_SDAREL16: 32,
        R_PPC_SECTOFF: 33,
        R_PPC_SECTOFF_LO: 34,
        R_PPC_SECTOFF_HI: 35,
        R_PPC_SECTOFF_HA: 36,
        R_PPC_COUNT: 37,
    }
}

impl ElfRelocationType
{
    fn to_rel_reloc(&self) -> RelRelocationType
    {
        // TODO: Some of the PLT or GOT relocation types might be mappable onto
        //       Rel relocation types
        match self {
            ElfRelocationType::R_PPC_NONE => RelRelocationType::R_PPC_NONE,
            ElfRelocationType::R_PPC_ADDR32 => RelRelocationType::R_PPC_ADDR32,
            ElfRelocationType::R_PPC_ADDR24 => RelRelocationType::R_PPC_ADDR24,
            ElfRelocationType::R_PPC_ADDR16 => RelRelocationType::R_PPC_ADDR16,
            ElfRelocationType::R_PPC_ADDR16_LO => RelRelocationType::R_PPC_ADDR16_LO,
            ElfRelocationType::R_PPC_ADDR16_HI => RelRelocationType::R_PPC_ADDR16_HI,
            ElfRelocationType::R_PPC_ADDR16_HA => RelRelocationType::R_PPC_ADDR16_HA,
            ElfRelocationType::R_PPC_ADDR14 => RelRelocationType::R_PPC_ADDR14,
            ElfRelocationType::R_PPC_ADDR14_BRTAKEN => RelRelocationType::R_PPC_ADDR14_BRTAKEN,
            ElfRelocationType::R_PPC_ADDR14_BRNTAKEN => RelRelocationType::R_PPC_ADDR14_BRNTAKEN,
            ElfRelocationType::R_PPC_REL24 => RelRelocationType::R_PPC_REL24,
            ElfRelocationType::R_PPC_REL14 => RelRelocationType::R_PPC_REL14,
            ElfRelocationType::R_PPC_REL14_BRTAKEN => RelRelocationType::R_PPC_REL14,
            ElfRelocationType::R_PPC_REL14_BRNTAKEN => RelRelocationType::R_PPC_REL14,
            ElfRelocationType::R_PPC_GOT16 => panic!(),
            ElfRelocationType::R_PPC_GOT16_LO => panic!(),
            ElfRelocationType::R_PPC_GOT16_HI => panic!(),
            ElfRelocationType::R_PPC_GOT16_HA => panic!(),
            ElfRelocationType::R_PPC_PLTREL24 => RelRelocationType::R_PPC_REL24,
            ElfRelocationType::R_PPC_COPY => panic!(),
            ElfRelocationType::R_PPC_GLOB_DAT => panic!(),
            ElfRelocationType::R_PPC_JMP_SLOT => panic!(),
            ElfRelocationType::R_PPC_RELATIVE => panic!(),
            ElfRelocationType::R_PPC_LOCAL24PC => panic!(),
            ElfRelocationType::R_PPC_UADDR32 => RelRelocationType::R_PPC_ADDR32,
            ElfRelocationType::R_PPC_UADDR16 => RelRelocationType::R_PPC_ADDR16,
            ElfRelocationType::R_PPC_REL32 => panic!(),
            ElfRelocationType::R_PPC_PLT32 => panic!(),
            ElfRelocationType::R_PPC_PLTREL32 => panic!(),
            ElfRelocationType::R_PPC_PLT16_LO => panic!(),
            ElfRelocationType::R_PPC_PLT16_HI => panic!(),
            ElfRelocationType::R_PPC_PLT16_HA => panic!(),
            // TODO: If I can learn how to compute SDABASE from the dol, then I can
            //       probably figure out a way to implement this one
            ElfRelocationType::R_PPC_SDAREL16 => panic!(),
            ElfRelocationType::R_PPC_SECTOFF => panic!(),
            ElfRelocationType::R_PPC_SECTOFF_LO => panic!(),
            ElfRelocationType::R_PPC_SECTOFF_HI => panic!(),
            ElfRelocationType::R_PPC_SECTOFF_HA => panic!(),
            ElfRelocationType::R_PPC_COUNT => panic!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
enum RelRelocationType
{
    R_PPC_NONE = 0,
    R_PPC_ADDR32 = 1,
    R_PPC_ADDR24 = 2,
    R_PPC_ADDR16 = 3,
    R_PPC_ADDR16_LO = 4,
    R_PPC_ADDR16_HI = 5,
    R_PPC_ADDR16_HA = 6,
    R_PPC_ADDR14 = 7,
    R_PPC_ADDR14_BRTAKEN = 8,
    R_PPC_ADDR14_BRNTAKEN = 9,
    R_PPC_REL24 = 10,
    R_PPC_REL14 = 11,
    R_DOLPHIN_NOP = 201,
    R_DOLPHIN_SECTION = 202,
    R_DOLPHIN_END = 203,
}


const SHN_ABS: u16 = 65521;
const SHN_COMMON: u16 = 65522;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RelocationKind<'a>
{
    ExternalSymbol(&'a str),
    InternalSymbol(u32 /* sec idx */, u32 /* offset */),
    AbsoluteSymbol(u32),
}

#[derive(Copy, Clone, Debug)]
struct Relocation<'a>
{
    kind: RelocationKind<'a>,
    addend: u32,
    offset: u32,
    type_: ElfRelocationType,
}

impl<'a> Relocation<'a>
{
    fn from_reloc(
        reloc: elf::reloc::Reloc,
        elf: &Elf<'a>,
        map_sec_index: impl Fn(u32) -> u32,
        map_bss_index: impl Fn(u32) -> u32,
    ) -> Relocation<'a>
    {
        let sym = elf.syms.get(reloc.r_sym).unwrap();
        Relocation {
            offset: reloc.r_offset as u32,
            addend: reloc.r_addend.unwrap_or(0) as u32,
            type_: ElfRelocationType::from_u32(reloc.r_type).unwrap(),
            kind: if sym.st_type() == elf::sym::STT_SECTION {
                RelocationKind::InternalSymbol(map_sec_index(sym.st_shndx as u32), 0)
            } else if sym.is_import() {
                RelocationKind::ExternalSymbol(elf.strtab.get(sym.st_name).unwrap().unwrap())
            } else if sym.st_shndx as u16 == SHN_COMMON {
                RelocationKind::InternalSymbol(map_bss_index(reloc.r_sym as u32), 0)
            } else if sym.st_shndx as u16 == SHN_ABS {
                RelocationKind::AbsoluteSymbol(sym.st_value as u32)
            } else {
                RelocationKind::InternalSymbol(
                    map_sec_index(sym.st_shndx as u32),
                    sym.st_value as u32
                )
            },
        }
    }

    fn is_locally_resolvable(
        &self,
        loc_sec: &LocatedSection,
        local_sym_table: &HashMap<&str, (RelSectionType, u32)>
    ) -> bool
    {
        let (known_static_addr, known_relative_addr) = match self.kind {
            RelocationKind::InternalSymbol(sec_idx, _) => {
                let sec_type = loc_sec.sibling_section_rel_sections[sec_idx as usize].unwrap();
                // The addr is never known for BSS sections
                (false, sec_type != RelSectionType::Bss)
            },
            RelocationKind::ExternalSymbol(sym_name) =>
                if local_sym_table.contains_key(&sym_name) {
                    (false, true)
                } else {
                    (true, false)
                },
            RelocationKind::AbsoluteSymbol(_) => (true, false),
        };

        match self.type_ {
            ElfRelocationType::R_PPC_NONE => true,
            ElfRelocationType::R_PPC_ADDR32 => known_static_addr,
            ElfRelocationType::R_PPC_ADDR24 => known_static_addr,
            ElfRelocationType::R_PPC_ADDR16 => known_static_addr,
            ElfRelocationType::R_PPC_ADDR16_LO => known_static_addr,
            ElfRelocationType::R_PPC_ADDR16_HI => known_static_addr,
            ElfRelocationType::R_PPC_ADDR16_HA => known_static_addr,
            ElfRelocationType::R_PPC_ADDR14 => known_static_addr,
            ElfRelocationType::R_PPC_ADDR14_BRTAKEN => known_static_addr,
            ElfRelocationType::R_PPC_ADDR14_BRNTAKEN => known_static_addr,
            ElfRelocationType::R_PPC_REL24 => known_relative_addr,
            ElfRelocationType::R_PPC_REL14 => known_relative_addr,
            ElfRelocationType::R_PPC_PLTREL24 => known_relative_addr,
            ElfRelocationType::R_PPC_REL32 => known_relative_addr,
            _ => panic!("Unimplemented relocation {:?}", self.type_),
        }
    }

    fn is_dol_relocation(&self, local_sym_table: &HashMap<&str, (RelSectionType, u32)>) -> bool
    {
        match self.kind {
            RelocationKind::ExternalSymbol(sym_name) => !local_sym_table.contains_key(sym_name),
            RelocationKind::AbsoluteSymbol(_) => true,
            RelocationKind::InternalSymbol(_, _) => false,
        }
    }


    fn to_rel_relocation(
        &self,
        offset: u16,
        loc_sec: &LocatedSection,
        rel_sections: &EnumMap<RelSectionType, SectionInfo>,
        local_sym_table: &HashMap<&str, (RelSectionType, u32)>,
        extern_sym_table: &HashMap<String, u32>,
    ) -> Result<RelRelocation>
    {
        let (section_index, symbol_offset) = match self.kind {
            RelocationKind::InternalSymbol(sec_index, offset) => {
                let sec_offset = loc_sec.sibling_section_offsets[sec_index as usize].unwrap();
                let sec_type = loc_sec.sibling_section_rel_sections[sec_index as usize].unwrap();
                (rel_sections[sec_type].rel_section_index.unwrap(), sec_offset + offset)
            },
            RelocationKind::ExternalSymbol(sym_name) => {
                if let Some((sec_type, offset)) = local_sym_table.get(sym_name) {
                    (rel_sections[*sec_type].rel_section_index.unwrap(), *offset)
                } else if let Some(offset) = extern_sym_table.get(sym_name) {
                    (0, *offset)
                } else {
                    // XXX Is there a more direct way to do this?
                    ensure!(false, UnresolvedSymbol { symbol_name: sym_name });
                    unreachable!()
                }
            },
            RelocationKind::AbsoluteSymbol(addr) => (0, addr),
        };

        Ok(RelRelocation {
            offset,
            relocation_type: self.type_.to_rel_reloc() as u8,
            section_index,
            symbol_offset: symbol_offset + self.addend,
        })
    }

    fn apply_relocation(
        &self,
        data: &[u8],
        self_offset: u32,
        loc_sec: &LocatedSection,
        rel_section_locations: &EnumMap<RelSectionType, Option<u32>>,
        locations_are_relative: bool,
        local_sym_table: &HashMap<&str, (RelSectionType, u32)>,
        extern_sym_table: &HashMap<String, u32>,
    ) -> Vec<u8>
    {
        let rel_addr;
        let abs_addr;
        match self.kind {
            RelocationKind::InternalSymbol(sec_index, offset) => {
                let sec_offset = loc_sec.sibling_section_offsets[sec_index as usize].unwrap();
                let sec_type = loc_sec.sibling_section_rel_sections[sec_index as usize].unwrap();

                rel_addr = Some((rel_section_locations[sec_type].unwrap()
                        + sec_offset
                        + offset
                        + self.addend) as i64);
                abs_addr = if !locations_are_relative {
                    Some(rel_addr.unwrap())
                } else {
                    None
                };
            },
            RelocationKind::ExternalSymbol(sym_name) => {
                if let Some((sec_type, offset)) = local_sym_table.get(sym_name) {
                    rel_addr = Some((rel_section_locations[*sec_type].unwrap()
                            + *offset
                            + self.addend) as i64);
                    abs_addr = if !locations_are_relative {
                        Some(rel_addr.unwrap())
                    } else {
                        None
                    };
                } else if let Some(offset) = extern_sym_table.get(sym_name) {
                    abs_addr = Some((*offset + self.addend) as i64);
                    rel_addr = if !locations_are_relative {
                        Some(abs_addr.unwrap())
                    } else {
                        None
                    };
                } else {
                    // We should have already filtered out any unresolved symbols
                    unreachable!("Symbol: {}", sym_name)
                }
            },
            RelocationKind::AbsoluteSymbol(addr) => {
                rel_addr = None;
                abs_addr = Some((addr + self.addend) as i64);
            },
        };
        let rel_addr = rel_addr.map(|addr| (addr - self_offset as i64));

        let read_instr = || u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        let bounds_check_and_mask = |len: u8, addr: i64| {
            // XXX Only len + 1 because this is a sign-extended value
            if addr > (1 << (len + 1)) - 1
                || addr < -1 << (len + 1)
                || addr as u64 & 0x3 != 0 {
                panic!()
            } else {
                (addr as u64 & ((1 << (len + 2)) - 1)) as u32
            }
        };

        match (self.type_, rel_addr, abs_addr) {
            (ElfRelocationType::R_PPC_NONE, _, _) => vec![],

            (ElfRelocationType::R_PPC_UADDR32, _, Some(addr)) |
            (ElfRelocationType::R_PPC_ADDR32, _, Some(addr)) |
            (ElfRelocationType::R_PPC_REL32, Some(addr), _) => {
                (addr as u32).to_be_bytes().to_vec()
            },
            (ElfRelocationType::R_PPC_ADDR24, _, Some(abs_addr)) => {
                let addr = bounds_check_and_mask(24, abs_addr);
                ((read_instr() & 0xfc000003) | addr).to_be_bytes().to_vec()
            },
            (ElfRelocationType::R_PPC_UADDR16, _, Some(abs_addr)) |
            (ElfRelocationType::R_PPC_ADDR16, _, Some(abs_addr)) if abs_addr < (1 << 16) - 1 =>
                (abs_addr as u32).to_be_bytes()[2..].to_vec(),
            (ElfRelocationType::R_PPC_ADDR16_LO, _, Some(abs_addr)) =>
                (abs_addr as u32).to_be_bytes()[2..].to_vec(),
            (ElfRelocationType::R_PPC_ADDR16_HI, _, Some(abs_addr)) =>
                (abs_addr as u32).to_be_bytes()[..2].to_vec(),
            (ElfRelocationType::R_PPC_ADDR16_HA, _, Some(abs_addr)) => {
                if abs_addr & 0x8000 == 0 {
                    (abs_addr as u32).to_be_bytes()[..2].to_vec()
                } else {
                    // Actually do the shift to prevent any chance of overflow
                    (((abs_addr >> 16) + 1) as u32).to_be_bytes()[2..].to_vec()
                }
            },
            (ElfRelocationType::R_PPC_REL14, Some(addr), _) |
            (ElfRelocationType::R_PPC_ADDR14, _, Some(addr)) => {
                let addr = bounds_check_and_mask(14, addr);
                ((read_instr() & 0xffff0003) | addr).to_be_bytes().to_vec()
            },
            (ElfRelocationType::R_PPC_REL14_BRTAKEN, Some(addr), _) |
            (ElfRelocationType::R_PPC_ADDR14_BRTAKEN, _, Some(addr)) => {
                let addr = bounds_check_and_mask(14, addr);
                ((read_instr() & 0xffdf0003) | addr | 1 << 21).to_be_bytes().to_vec()
            },
            (ElfRelocationType::R_PPC_REL14_BRNTAKEN, Some(addr), _) |
            (ElfRelocationType::R_PPC_ADDR14_BRNTAKEN , _, Some(addr)) => {
                let addr = bounds_check_and_mask(14, addr);
                ((read_instr() & 0xffdf0003) | addr).to_be_bytes().to_vec()
            },
            (ElfRelocationType::R_PPC_PLTREL24, Some(rel_addr), _) |
            (ElfRelocationType::R_PPC_REL24, Some(rel_addr), _) => {
                let addr = bounds_check_and_mask(24, rel_addr);
                ((read_instr() & 0xfc000003) | addr).to_be_bytes().to_vec()
            },
            a => panic!("Unimplemented relocation {:?}", a),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SymbolVis
{
    Default,
    Hidden,
    // Singleton,
    // Eliminate,
}

impl SymbolVis
{
    fn from_st_other(st_other: u8) -> SymbolVis
    {
        match elf::sym::st_visibility(st_other) {
            elf::sym::STV_DEFAULT => SymbolVis::Default,
            elf::sym::STV_HIDDEN => SymbolVis::Hidden,
            i@_ => panic!("Unsupported symbol visiblity: {} {}", i, elf::sym::visibility_to_str(i)),
        }
    }
}


#[derive(Clone, Debug)]
struct Section<'a>
{
    name: &'a str,
    data: &'a [u8],
    alignment: u8,
    is_executable: bool,
    is_bss: bool,
    exported_symbols: Vec<(&'a str, u32, SymbolVis)>,
    relocations: Vec<Relocation<'a>>,
}

impl<'a> Section<'a>
{
    fn from_section_header(
        sec_idx: usize,
        sh: &elf::section_header::SectionHeader,
        bytes: &'a [u8],
        elf: &Elf<'a>,
        map_sec_index: impl Fn(u32) -> u32,
        map_bss_index: impl Fn(u32) -> u32,
    ) -> Self
    {
        let mut relocations = elf.shdr_relocs.iter()
            .filter(|(idx, _)| elf.section_headers[*idx].sh_info == sec_idx as u32)
            .flat_map(|(_, reloc_section)| reloc_section.iter())
            .map(|reloc| Relocation::from_reloc(reloc, &elf, &map_sec_index, &map_bss_index))
            .collect::<Vec<_>>();
        relocations.sort_by_key(|reloc| reloc.offset);

        let exported_symbols = elf.syms.iter()
            .filter(|sym| sym.st_bind() == elf::sym::STB_GLOBAL ||
                          sym.st_bind() == elf::sym::STB_WEAK)
            .filter(|sym| sym.st_shndx == sec_idx)
            .map(|sym| {
                (
                    elf.strtab.get(sym.st_name).unwrap().unwrap(),
                    sym.st_value as u32,
                    SymbolVis::from_st_other(sym.st_other),
                )
            })
            .collect::<Vec<_>>();

        let data = if sh.sh_type == elf::section_header::SHT_NOBITS {
            &ZEROES[..sh.sh_size as usize]
        } else {
            &bytes[sh.sh_offset as usize..sh.sh_offset as usize + sh.sh_size as usize]
        };
        Section {
            name: elf.shdr_strtab.get(sh.sh_name).unwrap().unwrap(),
            data,
            alignment: sh.sh_addralign as u8,
            is_executable: sh.sh_flags as u32 & elf::section_header::SHF_EXECINSTR != 0,
            is_bss: sh.sh_type == elf::section_header::SHT_NOBITS,
            exported_symbols,
            relocations,

        }
    }

    fn from_common_symbol(
        sym: &elf::sym::Sym,
        elf: &Elf<'a>,
    ) -> Self
    {
        Section {
            name: elf.strtab.get(sym.st_name).unwrap().unwrap(),
            alignment: sym.st_value as u8,
            data: &ZEROES[..sym.st_size as usize],

            is_executable: false,
            is_bss: true,

            relocations: vec![],
            exported_symbols: if sym.st_bind() == elf::sym::STB_GLOBAL ||
                                sym.st_bind() == elf::sym::STB_WEAK {
                    vec![(
                        elf.strtab.get(sym.st_name).unwrap().unwrap(),
                        0,
                        SymbolVis::from_st_other(sym.st_other),
                    )]
                } else {
                    vec![]
                },
        }
    }

    fn size(&self) -> u32
    {
        self.data.len() as u32
    }

    fn section_type(&self) -> RelSectionType
    {
        if self.is_executable {
            RelSectionType::Text
        } else if self.is_bss {
            RelSectionType::Bss
        } else {
            RelSectionType::Data
        }
    }
}


#[derive(Clone, Debug)]
struct ObjectFile<'a>
{
    sections: Vec<Section<'a>>
}

impl<'a> ObjectFile<'a>
{
    fn from_elf(bytes: &'a [u8], elf: Elf<'a>) -> Self
    {
        let sec_indices_map = elf.section_headers.iter()
            .enumerate()
            .filter(|(_, sh)| (sh.sh_type == elf::section_header::SHT_PROGBITS
                                || sh.sh_type == elf::section_header::SHT_NOBITS)
                                && sh.sh_flags as u32 & elf::section_header::SHF_ALLOC != 0)
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();
        let bss_indices_map = elf.syms.iter()
            .enumerate()
            .filter(|(_, sym)| sym.st_shndx as u16 == SHN_COMMON)
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();

        let map_sec_index = |shndx| sec_indices_map.iter()
            .position(|i| *i == shndx)
            .unwrap() as u32;
        let map_bss_index = |shndx| bss_indices_map.iter()
            .position(|i| *i == shndx)
            .unwrap() as u32 + sec_indices_map.len() as u32;

        let mut sections = vec![];
        for (i, sh) in elf.section_headers.iter().enumerate() {
            if (sh.sh_type != elf::section_header::SHT_PROGBITS
                && sh.sh_type != elf::section_header::SHT_NOBITS)
                || sh.sh_flags as u32 & elf::section_header::SHF_ALLOC == 0 {

                continue
            }
            sections.push(Section::from_section_header(
                i,
                sh,
                bytes,
                &elf,
                &map_sec_index,
                &map_bss_index,
            ))
        }

        sections.extend(elf.syms.iter()
            .filter(|sym| sym.st_shndx as u16 == SHN_COMMON)
            .map(|sym| Section::from_common_symbol(&sym, &elf)));

        ObjectFile {
            sections,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Enum)]
enum RelSectionType
{
    Text,
    Data,
    Bss,
}

struct LocatedSection<'a>
{
    section: &'a Section<'a>,
    offset: u32,
    sibling_section_rel_sections: Rc<[Option<RelSectionType>]>,
    sibling_section_offsets: Rc<[Option<u32>]>,
}

impl<'a> std::ops::Deref for LocatedSection<'a>
{
    type Target = Section<'a>;
    fn deref(&self) -> &Self::Target
    {
        &self.section
    }
}

struct SectionInfo<'a>
{
    size: u32,
    alignment: u8,
    sections: Vec<LocatedSection<'a>>,
    rel_section_index: Option<u8>,
}

fn mmap_obj_files(obj_file_names: impl Iterator<Item = impl AsRef<Path>>)
    -> Result<Vec<(PathBuf, Mmap)>>
{
    let mut mmaps = vec![];
    for fname in obj_file_names {
        let fname = fname.as_ref();
        let f = File::open(fname)
            .with_context(|| OpenFile { filename: fname.to_path_buf() })?;
        let mmap = unsafe {
            MmapOptions::new()
                .map(&f)
                .with_context(|| OpenFile { filename: fname.to_path_buf() })?
        };
        mmaps.push((fname.to_path_buf(), mmap));
    }

    Ok(mmaps)
}

fn object_files_from_mmaps(mmaps: &[(PathBuf, Mmap)]) -> Result<Vec<ObjectFile>>
{
    let mut object_files = vec![];
    for (filename, mmap) in mmaps.iter() {
        match Object::parse(mmap).with_context(|| ObjectParsing { filename })? {
            Object::Elf(elf) => {
                object_files.push(ObjectFile::from_elf(mmap, elf));
            },
            Object::Archive(ar) => {
                for member_name in ar.members() {
                    let buf = ar.extract(member_name, mmap)
                        .with_context(|| ObjectParsing { filename })?;
                    let elf = Elf::parse(buf)
                        .with_context(|| ObjectParsing { filename })?;
                    object_files.push(ObjectFile::from_elf(buf, elf));
                }
            },
            _ => {
                ensure!(false, ObjectFormat { filename });
                unreachable!()
            }
        }
    }
    Ok(object_files)
}

/// Filter out sections that a) contain no public symbols and b) are unreferenced by sections that
/// contain public symbols or b) are (possibly transitively) referenced by sections which contains
/// public symbols.
fn filter_unused_sections<'a>(object_files: &'a [ObjectFile<'a>])
    -> Vec<(usize, usize, &'a Section<'a>)>
{
    // Naturally, we do this with a fixed point starting from the sections that do have public
    // symbols.
    let mut sections_to_keep = vec![];
    let mut sections_in_question = vec![];
    for (of_i, of) in object_files.iter().enumerate() {
        for (sec_i, sec) in of.sections.iter().enumerate() {
            if sec.exported_symbols.iter().any(|(_, _, vis)| *vis == SymbolVis::Default) {
                sections_to_keep.push((of_i, sec_i, sec));
            } else {
                sections_in_question.push((of_i, sec_i, sec));
            }
        }
    }

    // The actual fixed point: we iterate until nothing changes
    let mut prev_len = 0;
    while prev_len < sections_to_keep.len() {
        prev_len = sections_to_keep.len();

        sections_in_question.retain(|(of_qi, sec_qi, sec_q)| {

            // See if we any of the 'keep' secitons reference us
            let matches = sections_to_keep.iter()
                .map(|(of_ki, _, sec_k)| (&sec_k.relocations, of_ki))
                .flat_map(|(relocs_k, of_ki)| relocs_k.iter().zip(iter::repeat(of_ki)))
                .any(|(reloc, of_ki)| {
                    match reloc.kind {
                        RelocationKind::InternalSymbol(sec_idx, _) if of_qi == of_ki =>
                            // Check if any of sec_k's Internal relocs reference sec_q
                            sec_idx == *sec_qi as u32,
                        RelocationKind::ExternalSymbol(sn_k) if sn_k.starts_with("__start_") => {
                            sec_q.name == &sn_k[8..]
                        }
                        RelocationKind::ExternalSymbol(sn_k) if sn_k.starts_with("__stop_") => {
                            sec_q.name == &sn_k[7..]
                        },
                        RelocationKind::ExternalSymbol(sym_name_k) => {
                            // Check if any of sec_k's External relocs match sec_q's symbols
                            sec_q.exported_symbols
                                .iter()
                                .any(|(sym_name_q, _, _)| sym_name_k == *sym_name_q)
                        },
                        _ => false,
                    }
                });

            if matches {
                sections_to_keep.push((*of_qi, *sec_qi, sec_q));
                // Remove from sections_in_question if we found a match
                false
            } else {
                true
            }
        });
    }
    sections_to_keep
}

/// Group elf-sections into rel-sections by name so that elf-sections with identical names are
/// layed out next to each other in the rel-sections
///
/// Compute whether the merged rel sections must be executable along the way
fn group_elf_sections<'a>(
    sections: Vec<(usize, usize, &'a Section<'a>)>,
    convert_bss_to_data: bool,
)
    -> Vec<(RelSectionType, usize, usize, &'a Section<'a>)>
{
    let mut grouped_elf_sections = HashMap::new();
    for (of_i, sec_i, sec) in sections.iter() {

        if sec.section_type() == RelSectionType::Bss {
            if convert_bss_to_data {
                let (_, elf_sections) = grouped_elf_sections.entry("bss".to_string())
                    .or_insert_with(|| (RelSectionType::Data, vec![]));
                elf_sections.push((*of_i, *sec_i, *sec));
            }
            continue
        }

        let name = sec.name.to_string();
        let (sec_type, elf_sections) = grouped_elf_sections.entry(name)
            .or_insert_with(|| (sec.section_type(), vec![]));
        elf_sections.push((*of_i, *sec_i, *sec));

        if *sec_type != sec.section_type() {
            *sec_type = RelSectionType::Text;
        }
    }

    let mut grouped_elf_sections = grouped_elf_sections.values()
        .flat_map(|(sec_type, elf_sections)| iter::repeat(*sec_type).zip(elf_sections.iter()))
        .map(|(sec_type, (of_i, sec_i, sec))| (sec_type, *of_i, *sec_i, *sec))
        .collect::<Vec<_>>();

    grouped_elf_sections.sort_by_key(|(_, _, _, sec)| sec.name);

    if !convert_bss_to_data {
        // We've split apart the bss and non-bss sections when we did the grouping, so build an
        // iterator that captures both
        let bss_sections_iter = sections.iter()
            .filter(|(_of_i, _sec_i, sec)| sec.section_type() == RelSectionType::Bss)
            .map(|(of_i, sec_i, sec)| (RelSectionType::Bss, *of_i, *sec_i, *sec));
        grouped_elf_sections.extend(bss_sections_iter);
    }
    grouped_elf_sections
}

fn build_rel_sections<'a>(
    object_files: &'a [ObjectFile<'a>],
    grouped_sections: Vec<(RelSectionType, usize, usize, &'a Section<'a>)>,
) -> EnumMap<RelSectionType, SectionInfo<'a>>
{
    // Compute the offset for each Elf section in its Rel section and, along the way, the size of
    // each Rel section.

    let mut object_file_section_offsets = object_files.iter()
        .map(|of| of.sections.iter().map(|_| None).collect::<Vec<_>>())
        .map(|v| v.into_boxed_slice().into())
        .collect::<Vec<Rc<[_]>>>();
    let mut object_file_section_types = object_files.iter()
        .map(|of| of.sections.iter().map(|_| None).collect::<Vec<_>>())
        .map(|v| v.into_boxed_slice().into())
        .collect::<Vec<Rc<[_]>>>();

    let mut curr_offsets = EnumMap::new();
    for &(sec_type, of_i, sec_i, sec) in grouped_sections.iter() {
        Rc::get_mut(&mut object_file_section_types[of_i]).unwrap()[sec_i] = Some(sec_type);

        let o = align_to(curr_offsets[sec_type], sec.alignment);
        curr_offsets[sec_type] = o + sec.size();
        Rc::get_mut(&mut object_file_section_offsets[of_i]).unwrap()[sec_i] = Some(o);
    }

    let mut curr_index = 0;
    let mut rel_sections: EnumMap<RelSectionType, _> = (|sec_type| {
        let size = curr_offsets[sec_type];
        SectionInfo {
            size,
            alignment: 0,
            sections: vec![],
            rel_section_index: if size > 0 {
                    curr_index += 1;
                    // NOTE We want the first section to be 1, not 0
                    Some(curr_index)
                } else {
                    None
                },
        }
    }).into();

    for &(sec_type, of_i, sec_i, sec) in grouped_sections.iter() {
        let sec_info = &mut rel_sections[sec_type];
        if sec_info.alignment < sec.alignment {
            sec_info.alignment = sec.alignment;
        }
        sec_info.sections.push(LocatedSection {
            section: sec,
            offset: object_file_section_offsets[of_i][sec_i].unwrap(),
            sibling_section_rel_sections: object_file_section_types[of_i].clone(),
            sibling_section_offsets: object_file_section_offsets[of_i].clone(),
        });
    }
    rel_sections
}

fn build_local_symbol_table<'a, 'b: 'a>(
    rel_sections: &'a EnumMap<RelSectionType, SectionInfo>,
    section_boundary_symbol_names: &'b mut HashMap<&'a str, (String, String)>,
) -> Result<HashMap<&'b str, (RelSectionType, u32)>>
{
    *section_boundary_symbol_names = rel_sections.values()
        .flat_map(|rs| rs.sections.iter())
        .map(|sec| sec.name)
        .map(|name| (name, (format!("__start_{}", name), format!("__stop_{}", name))))
        .collect::<HashMap<&str, (String, String)>>();

    let mut local_sym_table = HashMap::new();

    // Populate the local symbol table
    for (sec_type, rs) in rel_sections.iter() {
        for loc_sec in rs.sections.iter() {
            for (sym_name, sym_offset, _) in loc_sec.exported_symbols.iter() {
                let o = local_sym_table.insert(*sym_name, (sec_type, loc_sec.offset + sym_offset));
                // TODO: If we have a STV_SINGLETON symbol, we would actually want to not error
                // out, but instead keep only one of the two symbols
                ensure!(o.is_none(), DuplicateSymbol { symbol_name: *sym_name });
            }

            let (start_name, stop_name) = section_boundary_symbol_names
                .get(loc_sec.name)
                .unwrap();

            if sec_type != RelSectionType::Bss {
                // NOTE because we grouped the sections by name earlier, a simple min/max works here
                local_sym_table.entry(&start_name)
                    .and_modify(|(_, o)| *o = std::cmp::min(*o, loc_sec.offset))
                    .or_insert((sec_type, loc_sec.offset));

                local_sym_table.entry(&stop_name)
                    .and_modify(|(_, o)| *o = std::cmp::max(*o, loc_sec.offset + loc_sec.size()))
                    .or_insert((sec_type, loc_sec.offset + loc_sec.size()));
            }
        }
    }

    Ok(local_sym_table)
}

fn write_relocated_section_data(
    mut addr: u32,
    rel_sections: &EnumMap<RelSectionType, SectionInfo>,
    rel_section_locations: &EnumMap<RelSectionType, Option<u32>>,
    locations_are_relative: bool,
    local_sym_table: &HashMap<&str, (RelSectionType, u32)>,
    extern_sym_table: &HashMap<String, u32>,
    output_file_name: &Path,
    mut output_file: impl Write + Seek,
) -> Result<()>
{

    for (sec_type, rs) in rel_sections.iter() {
        if sec_type == RelSectionType::Bss {
            continue
        }

        // Ensure the whole section is properly aligned
        let aligned_addr = align_to(addr, rs.alignment);
        output_file.write_all(&[0u8; 64][..(aligned_addr - addr) as usize]).unwrap();
        addr = aligned_addr;

        for loc_sec in rs.sections.iter() {
            let data = loc_sec.data;

            let aligned_addr = align_to(addr, loc_sec.alignment);

            output_file.write_all(&[0u8; 64][..(aligned_addr - addr) as usize]).unwrap();

            let mut prev_offset = 0;
            for reloc in loc_sec.relocations.iter() {
                if locations_are_relative && !reloc.is_locally_resolvable(&loc_sec, &local_sym_table) {
                    continue
                }
                assert!(prev_offset <= reloc.offset as usize);

                output_file.write_all(&data[prev_offset..reloc.offset as usize])
                    .with_context(|| WriteFile { filename: output_file_name })?;

                let relocated_bytes = reloc.apply_relocation(
                    &data[reloc.offset as usize..],
                    rel_section_locations[sec_type].unwrap() + loc_sec.offset + reloc.offset,
                    loc_sec,
                    &rel_section_locations,
                    locations_are_relative,
                    &local_sym_table,
                    &extern_sym_table
                );
                output_file.write_all(&relocated_bytes[..])
                    .with_context(|| WriteFile { filename: output_file_name })?;

                prev_offset = reloc.offset as usize + relocated_bytes.len();
            }
            output_file.write_all(&data[prev_offset..])
                .with_context(|| WriteFile { filename: output_file_name })?;

            addr = aligned_addr + data.len() as u32;

        }
    }
    Ok(())
}


pub fn link_obj_files_to_rel<'a>(
    obj_file_names: impl Iterator<Item = impl AsRef<Path>>,
    extern_sym_table: &HashMap<String, u32>,
    output_file_name: impl AsRef<Path>,
) -> Result<()>
{

    let mmaps = mmap_obj_files(obj_file_names)?;
    let object_files = object_files_from_mmaps(&mmaps)?;

    let sections_to_keep = filter_unused_sections(&object_files);
    // TODO: Print the names of the sections we're keeping, for the sake of debugging the resulting
    //       binary's size

    let grouped_sections = group_elf_sections(sections_to_keep, false);
    let rel_sections = build_rel_sections(&object_files, grouped_sections);

    let mut section_boundary_symbol_names = HashMap::new();
    let local_sym_table = build_local_symbol_table(&rel_sections, &mut section_boundary_symbol_names)?;

    // Build the lists of relocations that will be included in the REL
    let mut dol_relocations = EnumMap::<_, Vec<_>>::new();
    let mut dol_curr_offsets = EnumMap::new();
    let mut self_relocations = EnumMap::new();
    let mut self_curr_offsets = EnumMap::new();
    for (sec_type, rs)in rel_sections.iter() {
        for loc_sec in rs.sections.iter() {
            for reloc in loc_sec.relocations.iter() {
                if reloc.is_locally_resolvable(&loc_sec, &local_sym_table) {
                    continue
                }

                let (curr_offset, relocations) = if reloc.is_dol_relocation(&local_sym_table) {
                    (
                        &mut dol_curr_offsets[sec_type],
                        &mut dol_relocations[sec_type]
                    )

                } else {
                    (
                        &mut self_curr_offsets[sec_type],
                        &mut self_relocations[sec_type]
                    )
                };

                let mut relative_offset = loc_sec.offset + reloc.offset - *curr_offset;
                while relative_offset > 0xFFFF {
                    relocations.push(RelRelocation {
                        offset: 0xFFFF,
                        relocation_type: RelRelocationType::R_DOLPHIN_NOP as u8,
                        section_index: 0,
                        symbol_offset: 0,
                    });
                    relative_offset -= 0xFFFF;
                }

                *curr_offset = loc_sec.offset + reloc.offset;

                relocations.push(reloc.to_rel_relocation(
                    relative_offset as u16,
                    loc_sec,
                    &rel_sections,
                    &local_sym_table,
                    &extern_sym_table,
                )?);

            }
        }

    }

    let section_count = 1 + rel_sections.values()
        .map(|rs| rs.rel_section_index.is_some() as u32)
        .sum::<u32>();

    let sections_table_size = section_count * 8;

    let has_dol_relocs = dol_relocations.values().any(|v| v.len() > 0);
    let has_self_relocs = self_relocations.values().any(|v| v.len() > 0);
    let imports_table_size = (has_dol_relocs as u32 + has_self_relocs as u32) * 8;

    // Build the imports & relocations tables first to make calculating the latter's size easier
    let mut relocs_table = vec![];
    let mut imports_table = vec![];

    let self_module_id = 1024;

    for (module_id, relocs) in &[(self_module_id, self_relocations), (0, dol_relocations)] {
        let relocs_table_start_size = relocs_table.len() as u32;

        for (sec_type, relocations) in relocs.iter() {
            if relocations.len() > 0 {
                let i = rel_sections[sec_type].rel_section_index.unwrap();
                relocs_table.push(RelRelocation::start_section_entry(i));
            }
            relocs_table.extend(relocations.iter().cloned());
        }

        // Did we actually have anything to push?
        if relocs.values().any(|v| v.len() > 0) {
            relocs_table.push(RelRelocation::end_relocations_entry());

            imports_table.push(RelImport {
                module_id: *module_id,
                relocations_offset: 0x40
                    + sections_table_size
                    + imports_table_size
                    + relocs_table_start_size * 8,
            })
        }
    }

    let relocs_table_size = relocs_table.len() as u32 * 8;

    let rel_header = RelHeader {
        module_id: self_module_id,

        next_module_link: 0,
        prev_module_link: 0,

        section_count,
        section_table_offset: 0x40,

        module_name_offset: 0,
        module_name_size: 0,

        version: 1,

        bss_size: rel_sections[RelSectionType::Bss].size,

        reloc_table_offset: 0x40 + sections_table_size + imports_table_size,
        import_table_offset: 0x40 + sections_table_size,
        import_table_size: imports_table_size,

        prolog_function_section: local_sym_table
            .get("__rel_prolog")
            .and_then(|(sec_type, _)| rel_sections[*sec_type].rel_section_index)
            .unwrap_or(0),
        epilog_function_section: local_sym_table
            .get("__rel_epilog")
            .and_then(|(sec_type, _)| rel_sections[*sec_type].rel_section_index)
            .unwrap_or(0),
        unresolved_function_section: local_sym_table
            .get("__rel_unresloved")
            .and_then(|(sec_type, _)| rel_sections[*sec_type].rel_section_index)
            .unwrap_or(0),

        padding: 0,

        prolog_function_offset: local_sym_table
            .get("__rel_prolog")
            .map(|(_, offset)| *offset)
            .unwrap_or(0),
        epilog_function_offset: local_sym_table
            .get("__rel_epilog")
            .map(|(_, offset)| *offset)
            .unwrap_or(0),
        unresolved_function_offset: local_sym_table
            .get("__rel_unresolved")
            .map(|(_, offset)| *offset)
            .unwrap_or(0),
    };

    let mut size_accum = 0x40
            + sections_table_size
            + imports_table_size
            + relocs_table_size;

    let mut sections_table = Vec::with_capacity(section_count as usize);
    sections_table.push(RelSectionInfo { offset: 0, size: 0, is_executable: false });
    let mut rel_section_locations = EnumMap::new();
    for (sec_type, rs) in rel_sections.iter() {
        if rs.size == 0 {
            continue
        }
        rel_section_locations[sec_type] = if sec_type == RelSectionType::Bss {
            None
        } else {
            size_accum = align_to(size_accum, rs.alignment);
            let o = size_accum;
            size_accum += rs.size;
            Some(o)
        };
        sections_table.push(RelSectionInfo {
            offset: rel_section_locations[sec_type].unwrap_or(0),
            is_executable: sec_type == RelSectionType::Text,
            size: rs.size,
        });
    }

    let output_file_name = output_file_name.as_ref();
    let mut output_file = File::create(output_file_name)
        .with_context(|| WriteFile { filename: output_file_name })?;
    output_file.iowrite_with(rel_header, scroll::BE)
        .with_context(|| WriteFile { filename: output_file_name })?;
    for section in sections_table {
        output_file.iowrite_with(section, scroll::BE)
            .with_context(|| WriteFile { filename: output_file_name })?;
    }
    for import in imports_table {
        output_file.iowrite_with(import, scroll::BE)
            .with_context(|| WriteFile { filename: output_file_name })?;
    }
    for reloc in relocs_table {
        output_file.iowrite_with(reloc, scroll::BE)
            .with_context(|| WriteFile { filename: output_file_name })?;
    }


    let pos = output_file.seek(SeekFrom::Current(0)).unwrap() as u32;

    // Write actual section data
    write_relocated_section_data(
        pos,
        &rel_sections,
        &rel_section_locations,
        true, /* locations_are_relative */
        &local_sym_table,
        &extern_sym_table,
        output_file_name,
        &mut output_file,
    )?;

    // Ensure the file length is a multiple of 32 so it loads currectly from the GC disc
    let pos = output_file.seek(SeekFrom::Current(0)).unwrap() as u32;
    let aligned_pos = align_to(pos, 32u8);
    output_file.write_all(&[0u8; 32][..(aligned_pos - pos) as usize]).unwrap();

    Ok(())
}

pub fn link_obj_files_to_bin<'a>(
    obj_file_names: impl Iterator<Item = impl AsRef<Path>>,
    load_addr: u32,
    extern_sym_table: &HashMap<String, u32>,
    output_file_name: impl AsRef<Path>,
) -> Result<Vec<(String, u32)>>
{

    let mmaps = mmap_obj_files(obj_file_names)?;
    let object_files = object_files_from_mmaps(&mmaps)?;

    let sections_to_keep = filter_unused_sections(&object_files);

    let grouped_sections = group_elf_sections(sections_to_keep, true);
    let rel_sections = build_rel_sections(&object_files, grouped_sections);

    let mut section_boundary_symbol_names = HashMap::new();
    let local_sym_table = build_local_symbol_table(&rel_sections, &mut section_boundary_symbol_names)?;

    for rs in rel_sections.values() {
        for loc_sec in rs.sections.iter() {
            for reloc in loc_sec.relocations.iter() {
                match reloc.kind {
                    RelocationKind::ExternalSymbol(sym_name) => {
                        ensure!(
                            local_sym_table.contains_key(sym_name)
                                || extern_sym_table.contains_key(sym_name),
                            UnresolvedSymbol { symbol_name: sym_name }
                        );
                    }
                    RelocationKind::InternalSymbol(_, _) => (),
                    RelocationKind::AbsoluteSymbol(_) => (),
                }
            }
        }
    }

    let mut curr_addr = load_addr;
    let mut rel_section_locations = EnumMap::new();
    for (sec_type, rs) in rel_sections.iter() {
        curr_addr = align_to(curr_addr, rs.alignment);
        rel_section_locations[sec_type] = Some(curr_addr);
        curr_addr += rs.size;
    }

    let output_file_name = output_file_name.as_ref();
    let mut output_file = File::create(output_file_name)
        .with_context(|| WriteFile { filename: output_file_name })?;

    // Write actual section data
    write_relocated_section_data(
        load_addr,
        &rel_sections,
        &rel_section_locations,
        false, /* locations_are_relative */
        &local_sym_table,
        &extern_sym_table,
        output_file_name,
        &mut output_file,
    )?;

    let mut sorted_symbols = local_sym_table.iter()
        .filter(|(n, _)| !(n.starts_with("__start_") || n.starts_with("__stop_")))
        .map(|(n, (sec_type, off))| (rel_section_locations[*sec_type].unwrap() + off, n))
        .collect::<Vec<_>>();
    sorted_symbols.sort_by_key(|(addr, _)| *addr);

    Ok(sorted_symbols.iter()
       .map(|(addr, sym_name)| (sym_name.to_string(), *addr))
            .collect()
    )
}

pub fn parse_symbol_table(
    fname: &Path,
    lines: impl Iterator<Item = std::io::Result<String>>,
) -> Result<HashMap<String, u32>>
{
    let mut sym_table = HashMap::new();
    for (line_number, line) in lines.enumerate() {
        let line = line
            .with_context(|| SymTableIO { filename: fname, line_number })?;

        if line.trim().len() == 0 {
            continue
        }

        let mut it = line.splitn(2, ' ');

        let addr = it.next()
            .with_context(|| SymTableWrongNumberOfComponenets { filename: fname, line_number })?;
        let name = it.next()
            .with_context(|| SymTableWrongNumberOfComponenets { filename: fname, line_number })?;

        ensure!(
            it.next().is_none(),
            SymTableWrongNumberOfComponenets { filename: fname, line_number }
        );

        let addr = u32::from_str_radix(addr.trim_start_matches("0x"), 16)
            .with_context(|| SymTableAddrParsing { filename: fname, line_number })?;

        let o = sym_table.insert(name.to_string(), addr);
        ensure!(o.is_none(), SymTableDuplicateEntry { filename: fname, line_number });
    }

    Ok(sym_table)
}

pub fn read_symbol_table(fname: impl AsRef<Path>)
    -> Result<HashMap<String, u32>>
{
    let fname = fname.as_ref();

    let file = File::open(fname)
        .with_context(|| OpenFile { filename: fname })?;
    let file = BufReader::new(file);

    parse_symbol_table(fname, file.lines())
}

fn align_to(x: impl Into<u32>, alignment: impl Into<u32>) -> u32
{
    let x = x.into();
    let alignment = alignment.into();
    if alignment == 0 {
        x
    } else {
        (x + (alignment - 1)) & !(alignment - 1)
    }
}

#[test]
fn test_external_symbol_table()
{
    let extern_sym_table = read_symbol_table("test_data/symbols.map").unwrap();
    assert_eq!(extern_sym_table["printf"], 0x80001230);
    assert_eq!(extern_sym_table["FindWidget__9CGuiFrameCFPCc"], 0x80002230);
}

#[test]
fn test_read_objects()
{
    let mmaps = mmap_obj_files(["test_data/func_a.o", "test_data/func_b.o"].iter()).unwrap();
    let object_files = object_files_from_mmaps(&mmaps).unwrap();

    assert_eq!(object_files.len(), 2);

    assert_eq!(object_files[0].sections.len(), 3);
    assert_eq!(object_files[1].sections.len(), 6);

}

#[test]
fn test_filter_sections()
{
    let mmaps = mmap_obj_files(["test_data/func_a.o", "test_data/func_b.o"].iter()).unwrap();
    let object_files = object_files_from_mmaps(&mmaps).unwrap();
    let sections_to_keep = filter_unused_sections(&object_files);
}
