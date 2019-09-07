use crate::{cpp_field, cpp_method};
use crate::rstl::{WString, Vector};

use core::ptr;

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

pub enum CWorldState { }
impl CWorldState
{
    #[cpp_method(CWorldState::SetDesiredAreaAssetId(unsigned int))]
    pub unsafe fn set_desired_area_asset_id(this: *mut CWorldState, id: u32)
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

    #[cpp_method(CGameState::SetCurrentWorldId(unsigned int))]
    pub unsafe fn set_current_world_id(this: *mut CGameState, id: u32)
    { }

    #[cpp_method(CGameState::GetCurrentWorldState(void) const)]
    pub unsafe fn get_current_world_state(this: *mut CGameState) -> *mut CWorldState
    { }
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

pub type EScriptObjectMessage = u32;
pub type TAreaId = u32;
pub type TEditorId = u32;
pub type TUniqueId = u32;

#[repr(C)]
pub struct SConnection
{
    // TODO: These are actually enums
    pub state: u32,
    pub msg: EScriptObjectMessage,
    pub obj_id: u32,
}

pub enum IVisitor {}

#[repr(C)]
pub struct CEntityVTable<This>
{
    pub unknown0: u32,
    pub unknown1: u32,
    pub dtor: extern "C" fn(*mut This, *mut IVisitor),
    // Accept__12CScriptRelayFR8IVisitor
    pub accept: extern "C" fn(*mut This),
    // PreThink__7CEntityFfR13CStateManager
    pub pre_think: extern "C" fn(*mut This, *mut CStateManager),
    // Think__12CScriptRelayFfR13CStateManager
    pub think: extern "C" fn(*mut This, *mut CStateManager),
    // AcceptScriptMsg__12CScriptRelayF20EScriptObjectMessage9TUniqueIdR13CStateManager
    pub accept_script_msg: extern "C" fn(*mut This, EScriptObjectMessage, TUniqueId, *mut CStateManager),
    // SetActive__7CEntityFb
    pub set_active: extern "C" fn(*mut This, u8),
}


#[repr(C)]
pub struct CEntity
{
    pub vtable: *const CEntityVTable<CEntity>,
    pub area_id: TAreaId,
    pub unique_id: TUniqueId,
    pub editor_id: TEditorId,
    pub connections: Vector<SConnection>,
    // TODO This a actually a bit field
    pub status: u8,
}

pub extern "C" fn entity_empty_accept_impl<T>(this: *mut T, visitor: *mut IVisitor)
{
    // TODO: Load visitor's vtable, and then call it's entry for
    // Visit__20TCastToPtr<7CWeapon>FR7CEntity (0x9 * 4?)
    unsafe {
        let vtable = ptr::read(visitor as *mut *mut extern "C" fn(*mut IVisitor, *mut T));
        let func_ptr = ptr::read(vtable.offset(0x9));
        (func_ptr)(visitor, this)
    }
}

pub enum CArchitectureQueue { }

pub enum CMainFlow { }
impl CMainFlow
{
    #[cpp_method(CMainFlow::AdvanceGameState(CArchitectureQueue &))]
    pub unsafe fn advance_game_state(this: *mut CMainFlow, q: *mut CArchitectureQueue)
    { }

    #[cpp_method(CMainFlow::SetGameState(EClientFlowStates, CArchitectureQueue &))]
    pub unsafe fn set_game_state(this: *mut CMainFlow, state: i32, q: *mut CArchitectureQueue)
    { }

    cpp_field!(game_state: i32; ro_val @ 0x14);

    pub const CLIENT_FLOW_STATE_UNSPECIFIED: i32 = -1;
    pub const CLIENT_FLOW_STATE_PRE_FRONT_END: i32 = 7;
    pub const CLIENT_FLOW_STATE_FRONT_END: i32 = 8;
    pub const CLIENT_FLOW_STATE_GAME: i32 = 14;
    pub const CLIENT_FLOW_STATE_GAME_EXIT: i32 = 15;
}
