use reader_writer::FourCC;
use structs::{FstEntryFile, GcDisc, Resource, ResourceKind};

use crate::mlvl_wrapper::{MlvlArea, MlvlEditor};

use std::{
    collections::{HashMap, HashSet},
    ops::RangeFrom,
};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct ResourceKey<'a>
{
    pak_name: &'a [u8],
    kind: FourCC,
    id: u32,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct MreaKey<'a>
{
    pak_name: &'a [u8],
    room_id: u32,
}

type SclyPatch<'a, 'r> = dyn FnMut(&mut PatcherState, &mut MlvlArea<'a, '_, '_, '_>) -> Result<(), String> + 'r;
pub struct PrimePatcher<'a, 'r>
{
    file_patches: HashMap<&'r [u8], Box<dyn FnMut(&mut FstEntryFile) -> Result<(), String> + 'r>>,
    // TODO: Come up with a better data structure for this. A per PAK list of patches, for example.
    resource_patches: Vec<(ResourceKey<'r>, Box<dyn FnMut(&mut Resource) -> Result<(), String> + 'r>)>,
    scly_patches: Vec<(MreaKey<'r>, Vec<Box<SclyPatch<'a, 'r>>>)>,
}

pub struct PatcherState
{
    pub fresh_instance_id_range: RangeFrom<u32>,
}

impl<'a, 'r> PrimePatcher<'a, 'r>
{
    pub fn new() -> PrimePatcher<'a, 'r>
    {
        PrimePatcher {
            file_patches: HashMap::new(),
            resource_patches: Vec::new(),
            scly_patches: Vec::new(),
        }
    }

    pub fn add_file_patch<F>(&mut self, name: &'r [u8], f: F)
        where F: FnMut(&mut FstEntryFile) -> Result<(), String> + 'r
    {
        self.file_patches.insert(name, Box::new(f));
    }

    pub fn add_resource_patch<F>(&mut self, pak_name: &'r [u8], kind: FourCC, id: u32, f: F)
        where F: FnMut(&mut Resource) -> Result<(), String> + 'r
    {
        let key = ResourceKey { pak_name, kind, id, };
        self.resource_patches.push((key, Box::new(f)));
    }

    pub fn add_scly_patch<F>(&mut self, pak_name: &'r [u8], room_id: u32, f: F)
        where F: FnMut(&mut PatcherState, &mut MlvlArea<'a, '_, '_, '_>) -> Result<(), String> + 'r
    {
        let key = MreaKey { pak_name, room_id, };
        if let Some((_, v)) = self.scly_patches.iter_mut().find(|p| p.0 == key) {
            v.push(Box::new(f));
        } else {
            self.scly_patches.push((key, vec![Box::new(f)]));
        }
    }

    pub fn run(&mut self, gc_disc: &mut GcDisc<'a>) -> Result<(), String>
    {
        let mut patcher_state = PatcherState {
            fresh_instance_id_range: 0xDEADBABE..
        };

        let files_to_patch = self.file_patches.keys()
            .map(|k| *k)
            .chain(self.scly_patches.iter().map(|p| p.0.pak_name))
            .chain(self.resource_patches.iter().map(|p| p.0.pak_name))
            .collect::<HashSet<_>>();
        let files = gc_disc.file_system_table.fst_entries.iter_mut()
            .filter(|e| files_to_patch.contains(&e.name.to_bytes()));

        for fst_entry in files {
            let name = fst_entry.name.clone().into_owned();
            let name = name.to_bytes();

            if let Some(patch) = self.file_patches.get_mut(name) {
                fst_entry.guess_kind();
                patch(&mut fst_entry.file_mut().unwrap())?
            }

            let pak_patch_exists = self.resource_patches.iter()
                .map(|p| p.0.pak_name)
                .chain(self.scly_patches.iter().map(|p| p.0.pak_name))
                .any(|n| n == name);
            if !pak_patch_exists {
                continue;
            }

            fst_entry.guess_kind();
            let pak = match fst_entry.file_mut().unwrap() {
                structs::FstEntryFile::Pak(pak) => pak,
                _ => panic!(),
            };

            // Frequently when patching the scripting for a room, we want to modify both the MREA
            // for that room and the MLVL for the whole region at the same. The borrow checker
            // doesn't allow us to hold mutable references to both at the same time, so create a
            // copy on the stack to modify and then overwrite the canonical MLVL at the end of the
            // PAK.
            let scly_patch_exists = self.scly_patches.iter().any(|p| p.0.pak_name == name);
            let mut mlvl_editor = if scly_patch_exists {
                let mlvl = pak.resources.iter()
                    .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
                    .unwrap()
                    .kind.as_mlvl().unwrap().into_owned();
                Some(MlvlEditor::new(mlvl))
            } else {
                None
            };

            let mut cursor = pak.resources.cursor();
            while cursor.peek().is_some() {
                let mut cursor = cursor.cursor_advancer();
                let res_key = ResourceKey {
                    pak_name: name,
                    kind: cursor.peek().unwrap().fourcc(),
                    id: cursor.peek().unwrap().file_id,
                };

                if let Some((_, patch)) = self.resource_patches.iter_mut().find(|p| p.0 == res_key) {
                    patch(cursor.value().unwrap())?;
                }

                let mrea_key = MreaKey {
                    pak_name: name,
                    room_id: cursor.peek().unwrap().file_id,
                };
                if let Some((_, patches)) = self.scly_patches.iter_mut().find(|p| p.0 == mrea_key) {
                    let mut mlvl_area = mlvl_editor.as_mut().unwrap().get_area(&mut cursor);
                    for patch in patches.iter_mut() {
                        patch(&mut patcher_state, &mut mlvl_area)?;
                    }
                }

                if cursor.peek().unwrap().fourcc() == b"MLVL".into() && mlvl_editor.is_some() {
                    let mlvl = mlvl_editor.take().unwrap().mlvl;
                    cursor.value().unwrap().kind = ResourceKind::Mlvl(mlvl);
                }
            }
        }
        Ok(())
    }
}
