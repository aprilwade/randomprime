
mod _rel_config {
    use serde::{Serialize, Deserialize};
    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    pub(crate) struct RelConfig
    {
        pub quickplay_mlvl: u32,
        pub quickplay_mrea: u32,
        // pub use_etag: bool,
        // pub etag: [u8; 16],
        // pub use_modified_date: bool,
        pub modified_date: [u8; 29],
    }
}
pub(crate) use self::_rel_config::RelConfig;
