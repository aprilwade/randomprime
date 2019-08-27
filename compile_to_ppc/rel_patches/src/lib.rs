#![feature(async_await)]
#![feature(macros_in_extern)]
#![feature(never_type)]
#![feature(try_blocks)]
#![no_std]

extern crate alloc;

use linkme::distributed_slice;

use primeapi::{patch_fn, prolog_fn};
use primeapi::mp1::{CGuiFrame, CGuiTextSupport, CGuiTextPane, CGuiWidget, CStringTable};
use primeapi::rstl::WString;

use alloc::boxed::Box;

use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

mod ipc_async;
mod sock_async;
mod async_utils;


static mut EVENT_LOOP: Option<Pin<Box<dyn Future<Output = !>>>> = None;

#[prolog_fn]
unsafe extern "C" fn setup_global_state()
{
    debug_assert!(EVENT_LOOP.is_none());
    EVENT_LOOP = Some(Pin::new_unchecked(Box::new(event_loop())));
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

#[patch_fn(kind = "return",
           target = "Draw__13CIOWinManagerCFv" + 0x124)]
unsafe extern "C" fn hook_every_frame()
{
    if ipc_async::running_on_dolphin() {
        return
    }

    let event_loop = EVENT_LOOP.as_mut().unwrap();
    let waker = crate::async_utils::empty_waker();
    let mut ctx = Context::from_waker(&waker);
    match event_loop.as_mut().poll(&mut ctx) {
        Poll::Pending => (),
        Poll::Ready(never) => never,
    }
}

use core::fmt::Write;

pub struct FmtCursor<'a>
{
    idx: usize,
    buf: &'a mut [u8]
}

impl<'a> FmtCursor<'a>
{
    pub fn new(buf: &'a mut [u8]) -> Self
    {
        FmtCursor {
            idx: 0,
            buf,
        }
    }

    pub fn position(&self) -> usize
    {
        self.idx
    }

    pub fn set_position(&mut self, pos: usize)
    {
        self.idx = pos
    }
}

impl<'a> core::fmt::Write for FmtCursor<'a>
{
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error>
    {
        if s.len() + self.idx > self.buf.len() {
            Err(core::fmt::Error)
        } else {
            self.buf[self.idx..self.idx + s.len()].copy_from_slice(s.as_bytes());
            self.idx += s.len();
            Ok(())
        }
    }
}

use primeapi::mp1::{CGameState, CPlayerState};
async fn event_loop() -> !
{
    let ss = sock_async::SockSystem::new().await;
    let addr = sock_async::SockAddr {
        len: 8,
        family: sock_async::AF_INET as u8,
        port: 9112,
        name: sock_async::INADDR_ANY,
        unused: Default::default(),
    };
    let mut server = ss.tcp_listen(&addr, 1).await.unwrap();
    loop {
        let mut client = server.accept().await.unwrap();
        loop {

            let game_state = CGameState::global_instance();
            if game_state.is_null() {
                crate::async_utils::stall_once().await;
                continue
            }

            let player_state_ptr = unsafe { CGameState::player_state(game_state) };
            if player_state_ptr.is_null() {
                crate::async_utils::stall_once().await;
                continue
            }
            let player_state = unsafe { ptr::read(player_state_ptr) };
            if player_state.is_null() {
                crate::async_utils::stall_once().await;
                continue
            }

            use generic_array::{GenericArray, typenum::U1024};
            let mut buf = ipc_async::Aligned32::new(GenericArray::<u8, U1024>::default());

            let res: crate::sock_async::Result<()> = try {
                let len = build_tracker_data_json(&mut buf, game_state, player_state)?;
                client.write(buf.as_inner_slice().truncate_to_len(len)).await?;
            };
            if res.is_err() {
                break
            }

            crate::async_utils::delay(crate::async_utils::milliseconds_to_ticks(5000)).await;
        }
    }
}

fn build_tracker_data_json(
    buf: &mut[u8],
    game_state: *mut CGameState,
    player_state: *mut CPlayerState,
) -> crate::sock_async::Result<usize>
{
    let mut f = FmtCursor::new(&mut buf[..]);
    write!(f, "{{\"play_time\":{},\"inventory\":{{", unsafe {CGameState::play_time(game_state) })
        .map_err(|_| crate::sock_async::Error::RANDOMPRIME)?;
    for i in 0..41 {
        let cap = unsafe { CPlayerState::get_item_capacity(player_state, i) };
        write!(f, "{}:{},", i, cap).map_err(|_| crate::sock_async::Error::RANDOMPRIME)?;
    }
    // Clear the trailing comma
    f.set_position(f.position() - 1);
    write!(f, "}}}}\n").map_err(|_| crate::sock_async::Error::RANDOMPRIME)?;
    Ok(f.position())
}
