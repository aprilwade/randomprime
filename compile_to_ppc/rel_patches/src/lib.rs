#![feature(extern_types)]
#![feature(macros_in_extern)]
#![no_std]

use linkme::distributed_slice;

use core::mem::MaybeUninit;

use primeapi::{cw_link_name, patch_fn};

#[repr(C)]
struct Mp1WString(MaybeUninit<[u32; 3]>);


extern "C" {
    // #[link_name = "wstring_l__4rstlFPCw"]
    #[cw_link_name(rstl::wstring_l(const wchar_t *))]
    fn wstring_ctor(this: *mut Mp1WString, s: *const u16) -> *mut Mp1WString;

    // #[link_name = "internal_dereference__Q24rstl66basic_string<w,Q24rstl14char_traits<w>,Q24rstl17rmemory_allocator>Fv"]
    #[cw_link_name(rstl::basic_string<wchar_t, rstl::char_traits<wchar_t>, rstl::rmemory_allocator>::internal_dereference(void))]
    fn wstring_dtor(this: *mut Mp1WString);

}

extern "C" {
    pub type CGuiFrame;
    pub type CGuiWidget;
    pub type CGuiTextPane;
    pub type CGuiTextSupport;

    // #[link_name = "FindWidget__9CGuiFrameCFPCc"]
    #[cw_link_name(CGuiFrame::FindWidget(const char *) const)]
    fn find_widget(this: *mut CGuiFrame, widget_name: *const u8) -> *mut CGuiWidget;

    // #[link_name = "SetText__15CGuiTextSupportFRCQ24rstl66basic_string<w,Q24rstl14char_traits<w>,Q24rstl17rmemory_allocator>"]
    #[cw_link_name(CGuiTextSupport::SetText(const rstl::basic_string<wchar_t, rstl::char_traits<wchar_t>, rstl::rmemory_allocator> &))]
    fn text_support_set_text(this: *mut CGuiTextSupport, s: *const Mp1WString);
}

impl CGuiFrame
{
    unsafe fn find_widget(this: *mut CGuiFrame, widget_name: *const u8) -> *mut CGuiWidget
    {
        find_widget(this, widget_name)
    }
}

impl CGuiTextPane
{
    unsafe fn text_support(this: *mut Self) -> *mut CGuiTextSupport
    {
        (this as usize + 0xd4) as *mut _
    }
}

impl CGuiTextSupport
{
    unsafe fn set_text(this: *mut Self, s: *const Mp1WString)
    {
        text_support_set_text(this, s)
    }
}


extern "C" {
    type CStringTable;
    static g_MainStringTable: *mut CStringTable;

    // #[link_name = "GetString__12CStringTableCFi"]
    #[cw_link_name(CStringTable::GetString(int) const)]
    fn string_table_get_string(this: *const CStringTable, idx: u32) -> *const u16;
}

impl CStringTable
{
    unsafe fn get_string(this: *const Self, idx: u32) -> *const u16
    {
        string_table_get_string(this, idx)
    }
}

#[patch_fn(kind = "call",
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c)]
unsafe extern "C" fn update_main_menu_text(frame: *mut CGuiFrame, widget_name: *const u8)
    -> *mut CGuiWidget
{
    let res = find_widget(frame, widget_name);

    let raw_string = CStringTable::get_string(g_MainStringTable, 110);
    let mut s = Mp1WString(MaybeUninit::uninit());
    wstring_ctor(&mut s, raw_string);

    for name in &[b"textpane_identifier\0".as_ptr(), b"textpane_identifierb\0".as_ptr()] {
        let widget = CGuiFrame::find_widget(frame, *name);
        let text_support = CGuiTextPane::text_support(widget as *mut CGuiTextPane);
        CGuiTextSupport::set_text(text_support, &s);
    }

    wstring_dtor(&mut s);

    res
}


