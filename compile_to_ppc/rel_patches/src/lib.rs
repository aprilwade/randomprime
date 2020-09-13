#![no_std]
#![type_length_limit = "14435114"]

extern crate alloc;

use alloc::boxed::Box;

use core::convert::Infallible;
use core::future::Future;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use primeapi::{patch_fn, prolog_fn, running_on_dolphin};
use primeapi::alignment_utils::Aligned32;
use primeapi::dol_sdk::dvd::DVDFileInfo;
use primeapi::mp1::{
    CArchitectureQueue, CGameState, CGuiFrame, CGuiTextSupport, CGuiTextPane, CGuiWidget,
    CMainFlow, CStringTable, CWorldState,
};
use primeapi::rstl::WString;

use futures::task::noop_waker;
use linkme::distributed_slice;

mod nintendont_sock;
mod tracker_state;
mod tracker_event_loop;

include!("../../patches_config.rs");
static mut REL_CONFIG: RelConfig = RelConfig {
    quickplay_mlvl: 0xFFFFFFFF,
    quickplay_mrea: 0xFFFFFFFF,
};

static mut EVENT_LOOP: Option<Pin<Box<dyn Future<Output = Infallible>>>> = None;

#[prolog_fn]
unsafe extern "C" fn setup_global_state()
{
    {
        let mut fi = if let Some(fi) = DVDFileInfo::new(b"rel_config.bin\0") {
            fi
        } else {
            return;
        };
        let config_size = fi.file_length() as usize;
        let mut recv_buf = alloc::vec![MaybeUninit::<u8>::uninit(); config_size + 63];
        let recv_buf = Aligned32::split_unaligned_prefix_mut(&mut recv_buf[..]).1;
        let recv_buf = &mut recv_buf[..(config_size + 31) & !31];
        {
            let _ = fi.read_async(recv_buf, 0, 0);
        }
        REL_CONFIG = ssmarshal::deserialize(&recv_buf[..config_size].assume_init())
            .unwrap().0;
    }

    if running_on_dolphin() {
        return
    }

    crate::nintendont_sock::SocketApi::global_instance();
    EVENT_LOOP = Some(Box::pin(crate::tracker_event_loop::event_loop()));
}


#[patch_fn(kind = "call",
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c)]
unsafe extern "C" fn update_main_menu_text(frame: *mut CGuiFrame, widget_name: *const u8)
    -> *mut CGuiWidget
{
    let res = CGuiFrame::find_widget(frame, widget_name);

    let raw_string = CStringTable::get_string(CStringTable::main_string_table(), 110);
    let s = WString::from_ucs2_str(raw_string);

    for name in &[b"textpane_identifier\0".as_ptr(), b"textpane_identifierb\0".as_ptr()] {
        let widget = CGuiFrame::find_widget(frame, *name);
        let text_support = CGuiTextPane::text_support_mut(widget as *mut CGuiTextPane);
        CGuiTextSupport::set_text(text_support, &s);
    }

    res
}

// Based on
// https://github.com/AxioDL/PWEQuickplayPatch/blob/249ae82cc20031fe99894524aefb1f151430bedf/Source/QuickplayModule.cpp#L150
#[patch_fn(kind = "call",
           target = "OnMessage__9CMainFlowFRC20CArchitectureMessageR18CArchitectureQueue" + 72)]
unsafe extern "C" fn quickplay_hook_advance_game_state(
    flow: *mut CMainFlow,
    q: *mut CArchitectureQueue
)
{
    static mut INIT: bool = false;
    if CMainFlow::game_state(flow) == CMainFlow::CLIENT_FLOW_STATE_PRE_FRONT_END  && !INIT {
        INIT = true;
        if REL_CONFIG.quickplay_mlvl != 0xFFFFFFFF {
            let game_state = CGameState::global_instance();
            CGameState::set_current_world_id(game_state, REL_CONFIG.quickplay_mlvl);
            let world_state = CGameState::get_current_world_state(game_state);
            CWorldState::set_desired_area_asset_id(world_state, REL_CONFIG.quickplay_mrea);
            CMainFlow::set_game_state(flow, CMainFlow::CLIENT_FLOW_STATE_GAME, q);
            return;
        }
    }
    CMainFlow::advance_game_state(flow, q)
}

#[patch_fn(kind = "return",
           target = "Draw__13CIOWinManagerCFv" + 0x124)]
unsafe extern "C" fn hook_every_frame()
{
    if running_on_dolphin() {
        return
    }

    // static mut COUNTER: u32 = 0;
    // COUNTER += 1;
    // if COUNTER == 4000 {
    //     COUNTER = 0;
    //     primeapi::printf(b"COUNTER reset\n\0".as_ptr());
    // }

    let event_loop = if let Some(event_loop) = EVENT_LOOP.as_mut() {
        event_loop
    } else {
        return
    };
    let waker = noop_waker();
    let mut ctx = Context::from_waker(&waker);
    match event_loop.as_mut().poll(&mut ctx) {
        Poll::Pending => (),
        Poll::Ready(never) => match never { },
    }
}
