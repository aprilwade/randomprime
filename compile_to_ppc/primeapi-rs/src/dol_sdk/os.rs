#[repr(C)]
pub struct OSModuleInfo
{
    pub id: u32,
    pub next: *mut OSModuleInfo,
    pub prev: *mut OSModuleInfo,

    pub sections_count: u32,
    pub section_info_offset: u32,
    pub name_offset: u32,
    pub name_size: u32,
    pub version: u32,
}

#[repr(C)]
pub union OffsetOrFuncPointer
{
    pub offset: u32,
    pub func_ptr: Option<unsafe extern "C" fn()>,
}

#[repr(C)]
pub struct OSModuleHeader
{
    pub mod_info: OSModuleInfo,
    pub bss_size: u32,

    pub reloc_table_offset: u32,
    pub import_table_offset: u32,
    pub import_table_size: u32,

    pub prolog_function_section: u8,
    pub epilog_function_section: u8,
    pub unresolved_function_section: u8,
    pub padding: u8,

    pub prolog_function: OffsetOrFuncPointer,
    pub epilog_function: OffsetOrFuncPointer,
    pub unresolved_function: OffsetOrFuncPointer,
}

extern "C" {
    pub fn OSLink(module: *mut OSModuleInfo, bss: *const u8) -> u8;
}

#[allow(non_snake_case)]
pub fn OSGetTime() -> u64
{
    extern "C" {
        fn OSGetTime() -> u64;
    }
    unsafe {
        OSGetTime()
    }
}
