pub use dol_symbol_table_macro::{
    mp1_100_symbol, mp1_101_symbol, mp1_102_symbol, mp1_pal_symbol, mp1_kor_symbol, mp1_jap_symbol,
    // mp1_trilogy_ntsc_j_symbol, mp1_trilogy_ntsc_u_symbol, mp1_trilogy_pal_symbol,
};

pub struct Mp1Symbol
{
    pub addr_0_00: Option<u32>,
    pub addr_0_01: Option<u32>,
    pub addr_0_02: Option<u32>,
    pub addr_pal: Option<u32>,
    pub addr_kor: Option<u32>,
    pub addr_jap: Option<u32>,
    // pub addr_trilogy_ntsc_u: Option<u32>,
    // pub addr_trilogy_ntsc_j: Option<u32>,
    // pub addr_trilogy_pal: Option<u32>,
}

#[macro_export]
macro_rules! mp1_symbol {
    ($syn_name:tt) => {
        $crate::Mp1Symbol {
            addr_0_00: $crate::mp1_100_symbol!($syn_name),
            addr_0_01: $crate::mp1_101_symbol!($syn_name),
            addr_0_02: $crate::mp1_102_symbol!($syn_name),
            addr_pal: $crate::mp1_pal_symbol!($syn_name),
            addr_kor: $crate::mp1_kor_symbol!($syn_name),
            addr_jap: $crate::mp1_jap_symbol!($syn_name),
            // addr_trilogy_ntsc_u: $crate::mp1_trilogy_ntsc_u_symbol!($syn_name),
            // addr_trilogy_ntsc_j: $crate::mp1_trilogy_ntsc_j_symbol!($syn_name),
            // addr_trilogy_pal: $crate::mp1_trilogy_pal_symbol!($syn_name),
        }
    }
}
