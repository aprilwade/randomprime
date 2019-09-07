#[cfg(not(target_arch = "powerpc"))]
use auto_struct_macros::auto_struct;

#[cfg_attr(not(target_arch = "powerpc"), auto_struct(Readable, Writable, FixedSize))]
#[repr(C)]
pub struct RelConfig
{
    pub quickplay_mlvl: u32,
    pub quickplay_mrea: u32,
}

