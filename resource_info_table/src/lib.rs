use reader_writer::FourCC;
use macro_file_proxy::macro_file_proxy_item;

#[derive(Copy, Clone, Debug)]
pub struct ResourceInfo
{
    pub long_name: &'static str,
    pub short_name: Option<&'static str>,
    pub res_id: u32,
    pub fourcc: FourCC,
    pub paks: &'static [&'static [u8]],
}

impl<'a, 'b> Into<(&'a [&'b [u8]], u32, FourCC)> for ResourceInfo
{
    fn into(self) -> (&'a [&'b [u8]], u32, FourCC)
    {
        (self.paks, self.res_id, self.fourcc)
    }
}

impl Into<(u32, FourCC)> for ResourceInfo
{
    fn into(self) -> (u32, FourCC)
    {
        (self.res_id, self.fourcc)
    }
}

impl<'a> Into<(&'a [u8], u32)> for ResourceInfo
{
    fn into(self) -> (&'a [u8], u32)
    {
        assert_eq!(self.paks.len(), 1);
        (self.paks[0], self.res_id)
    }
}


#[macro_export]
macro_rules! build_resource_info_macro {
    ($($long_name:tt, $res_id:expr, $fourcc:expr, $paks:expr $(,$short_name:tt)?;)+) => {
        #[macro_export]
        macro_rules! resource_info {
            $(
            ($long_name) => { $crate::ResourceInfo {
                long_name: $long_name,
                short_name: $crate::build_resource_info_macro!(SN $($short_name)?),
                res_id: $res_id,
                fourcc: reader_writer::FourCC::from_bytes($fourcc),
                paks: $paks,
            }};
            $(
            ($short_name) => { $crate::resource_info!($long_name) };
            )?
            )+
        }
    };
    (SN $short_name:expr) => { Some($short_name) };
    (SN) => { None };
}

// The following resource metadata is derived from PrimeWorldEditor's metadata:
// https://github.com/arukibree/PrimeWorldEditor/blob/master/resources/gameinfo/AssetNameMap32.xml
macro_file_proxy_item! { "resource_info.txt", build_resource_info_macro, ;}
