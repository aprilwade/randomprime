use crate::{cpp_field, cpp_method};
use crate::rstl::WString;

pub enum CGuiFrame { }
pub enum CGuiWidget { }
pub enum CGuiTextPane { }
pub enum CGuiTextSupport { }

impl CGuiFrame
{
    // FindWidget__9CGuiFrameCFPCc
    #[cpp_method(CGuiFrame::FindWidget(const char *) const)]
    pub unsafe fn find_widget(this: *const CGuiFrame, widget_name: *const u8) -> *mut CGuiWidget
    { }
}

impl CGuiTextPane
{
    cpp_field!(text_support: CGuiTextSupport; ptr @ 0xd4);
}

impl CGuiTextSupport
{
    // SetText__15CGuiTextSupportFRCQ24rstl66basic_string<w,Q24rstl14char_traits<w>,Q24rstl17rmemory_allocator>
    #[cpp_method(CGuiTextSupport::SetText(const rstl::basic_string<wchar_t, rstl::char_traits<wchar_t>, rstl::rmemory_allocator> &))]
    pub unsafe fn set_text(this: *mut CGuiTextSupport, s: *const WString)
    { }
}

pub enum CStringTable { }

impl CStringTable
{
    // GetString__12CStringTableCFi
    #[cpp_method(CStringTable::GetString(int) const)]
    pub unsafe fn get_string(this: *const CStringTable, idx: u32) -> *const u16
    { }

    pub fn main_string_table() -> *mut Self
    {
        extern "C" {
            static g_MainStringTable: *mut CStringTable;
        }
        unsafe {
            g_MainStringTable
        }
    }
}

// #[repr(C)]
// #[derive(Copy, Clone, Debug)]
// struct CPowerUp
// {
//     amount: u32,
//     capacity: u32,
// }

pub enum CPlayerState { }
impl CPlayerState
{
    #[cpp_method(CPlayerState::GetItemCapacity(CPlayerState::EItemType) const)]
    pub unsafe fn get_item_capacity(this: *const CPlayerState, type_: i32) -> u32
    { }

    #[cpp_method(CPlayerState::IncrPickup(CPlayerState::EItemType, u32))]
    pub unsafe fn incr_pickup(this: *const CPlayerState, type_: i32, amount: u32)
    { }

    #[cpp_method(CPlayerState::DecrPickup(CPlayerState::EItemType, u32) const)]
    pub unsafe fn decr_pickup(this: *const CPlayerState, type_: i32, amount: u32)
    { }
}

pub enum CGameState { }
impl CGameState
{
    // TODO: I guess this should actually be shared_ptr
    cpp_field!(player_state: *mut *mut CPlayerState; val @ 0x98);
    cpp_field!(play_time: f64; val @ 0xa0);

    pub fn global_instance() -> *mut Self
    {
        extern "C" {
            static g_GameState: *mut CGameState;
        }
        unsafe {
            g_GameState
        }
    }
}

pub enum CStateManager { }
impl CStateManager
{
    cpp_field!(player_state: *mut CPlayerState; ptr @ 0x8b8);
}

#[repr(C)]
pub struct CHudMemoParams
{
    pub display_time: f32,
    pub clear_memo_window: u8,
    pub fadeout_only: u8,
    pub hint_memo: u8,
}

pub enum CSamusHud { }
impl CSamusHud
{
    #[cpp_method(CSamusHud::DisplayHudMemo(const wstring &, const SHudMemoInfo &))]
    pub unsafe fn display_hud_memo(s: *const WString, info: *const CHudMemoParams)
    { }
}

