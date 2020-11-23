use reader_writer::FourCC;

pub use resource_info_table_macro::resource_info;

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
