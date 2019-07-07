use resource_info_table::{
    resource_info, ResourceInfo
};

use macro_file_proxy::macro_file_proxy_item;

use std::env::args;
use std::u32;


macro_rules! build_resource_info_array {
    ($($long_name:tt, $res_id:expr, $fourcc:expr, $paks:expr $(, $short_name:tt)?;)+) => {
        const RESOURCE_INFO: &[$crate::ResourceInfo] = &[
            $(resource_info!($long_name),)+
        ];
    };
}

macro_file_proxy_item! { "resource_info.txt", build_resource_info_array, ;}

pub fn lookup_resource_info(res_id: u32) -> Option<ResourceInfo>
{
    RESOURCE_INFO.binary_search_by_key(&res_id, |res| res.res_id)
        .ok()
        .map(|id| RESOURCE_INFO[id])
}

fn main()
{
    let arg = args().nth(1).unwrap();
    let id = if arg.starts_with("0x") {
        u32::from_str_radix(&arg[2..], 16).unwrap()
    } else {
        u32::from_str_radix(&arg, 10).unwrap()
    };
    println!("{:#?}", lookup_resource_info(id));
}

