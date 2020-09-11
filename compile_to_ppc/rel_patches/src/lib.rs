#![no_std]
#![type_length_limit = "10794634"]

extern crate alloc;

use linkme::distributed_slice;
use ufmt::uwriteln;

use primeapi::{patch_fn, prolog_fn};
use primeapi::alignment_utils::Aligned32SliceMut;
use primeapi::dol_sdk::dvd::DVDFileInfo;
use primeapi::mp1::{
    CArchitectureQueue, CGameState, CGuiFrame, CGuiTextSupport, CGuiTextPane, CGuiWidget,
    CMainFlow, CStringTable, CWorldState,
};
use primeapi::rstl::WString;

use core::mem::MaybeUninit;

mod nintendont_sock;
mod tracker_state;

include!("../../patches_config.rs");
static mut REL_CONFIG: RelConfig = RelConfig {
    quickplay_mlvl: 0xFFFFFFFF,
    quickplay_mrea: 0xFFFFFFFF,
};

#[prolog_fn]
unsafe extern "C" fn setup_global_state()
{
    // core::writeln!(primeapi::Mp1Stdout, "-1");
    // primeapi::printf(b"-1\n\0".as_ptr());
    {
        // core::writeln!(primeapi::Mp1Stdout, "0");
        let mut fi = if let Some(fi) = DVDFileInfo::new(b"rel_config.bin\0") {
            // uwriteln!(primeapi::Mp1Stdout, "found").ok();
            // primeapi::printf(b"found\n\0".as_ptr());
            fi
        } else {
            // uwriteln!(primeapi::Mp1Stdout, "failed").ok();
            // primeapi::printf(b"failed\n\0".as_ptr());
            return;
        };
        let config_size = fi.file_length() as usize;
        let mut recv_buf = alloc::vec![MaybeUninit::<u8>::uninit(); config_size + 63];
        let mut recv_buf = Aligned32SliceMut::split_unaligned_prefix(&mut recv_buf[..]).1
            .truncate_to_len((config_size + 31) & !31);

        // core::writeln!(primeapi::Mp1Stdout, "1");
        let _ = fi.read_async(recv_buf.reborrow(), 0, 0);
        // core::writeln!(primeapi::Mp1Stdout, "2");

        // core::writeln!(primeapi::Mp1Stdout, "Before");

        // core::writeln!(primeapi::Mp1Stdout, "{:#?}", fi);

        // core::writeln!(primeapi::Mp1Stdout, "After");

        REL_CONFIG = ssmarshal::deserialize(&recv_buf.truncate_to_len(config_size).assume_init())
            .unwrap().0;
    }

    if running_on_dolphin() {
        return
    }

    // primeapi::printf(b"Before\n\0".as_ptr());
    crate::nintendont_sock::SocketApi::global_instance();
    // primeapi::printf(b"After\n\0".as_ptr());
    // primeapi::printf(b"After?\n\0".as_ptr());
    EVENT_LOOP = Some(Box::pin(event_loop()));
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

use alloc::boxed::Box;
use core::convert::Infallible;
use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, RawWakerVTable, RawWaker, Waker};

use async_utils::io::{AsyncRead, AsyncBufRead, AsyncWriteExt};

static EMPTY_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        |p| RawWaker::new(p, &EMPTY_RAW_WAKER_VTABLE),
        |_| (),
        |_| (),
        |_| (),
    );

fn empty_raw_waker() -> RawWaker
{
    RawWaker::new(ptr::null(), &EMPTY_RAW_WAKER_VTABLE)
}

pub fn empty_waker() -> Waker
{
    unsafe { Waker::from_raw(empty_raw_waker()) }
}


pub fn running_on_dolphin() -> bool
{
    unsafe { core::ptr::read(0xCD000004 as *mut u32) == 0xFFFFFFFF }
}

static mut EVENT_LOOP: Option<Pin<Box<dyn Future<Output = Infallible>>>> = None;

/* async fn event_loop() -> Infallible
{
    let sock_api = crate::nintendont_sock::SocketApi::global_instance();
    let mut server = sock_api.tcp_server(80, 1).unwrap();
    loop {
        let (mut stream, _) = match server.accept().await {
            Ok(s) => s,
            Err(e) => {
                unsafe {
                    primeapi::printf(b"accept failed: %d\n\0".as_ptr(), e);
                }
                continue
            }
        };
        match stream.write_all(b"testing").await {
            Ok(()) => (),
            Err(e) => {
                unsafe {
                    primeapi::printf(b"write_all failed: %d\n\0".as_ptr(), e);
                }
                continue
            }

        }
        let _ = stream.close().await;
    }
}*/

use nintendont_sock::SocketApi;
use futures::never::Never;
use pin_utils::pin_mut;
async fn event_loop() -> Never
{
    let mut server = SocketApi::global_instance().tcp_server(80, 1).unwrap();
    // let addr = nintendont_sock::SockAddr {
    //     len: 8,
    //     family: sock_async::AF_INET as u8,
    //     port: 80,
    //     name: sock_async::INADDR_ANY,
    //     unused: Default::default(),
    // };
    let mut queue = async_utils::FutureQueue::<_, generic_array::typenum::U5>::new();
    let (queue_poller, mut queue_pusher) = queue.split();
    // let mut server = ss.tcp_listen(&addr, 1).await.unwrap();
    let connect_fut = async {
        loop {
            // TODO: Allow multiple clients, maybe with a vec?
            //       Set a recv timeout on the client for 30s or 60s
            let (mut client, _addr) = server.accept().await.unwrap();

            let fut = Box::pin(async move {
                let (mut reader, mut writer) = client.split();
                let handler = mini_http_server::HttpRequestHandler::new()
                    .reader(&mut reader)
                    .writer(&mut writer)
                    .fs(DvdFileSystem)
                    // .ws(|_, _, _, w| -> Result<futures::future::Ready<Result<(), ()>>, _> { Err(w) });
                    .ws(|_uri, key, r, w| {
                        Ok(websocket_logic(r, w, key))
                    });

                match handler.handle_http_request().await {
                    Ok(_) => {
                        primeapi::dbg!("successful http request");
                    },
                    Err(mini_http_server::HttpRequestError::ReaderIO(e)) => {
                        primeapi::dbg!("Reader IO Error", e);
                    },
                    Err(mini_http_server::HttpRequestError::WriterIO(e)) => {
                        primeapi::dbg!("Writer IO Error", e);
                    },
                    Err(mini_http_server::HttpRequestError::Internal) => {
                        primeapi::dbg!("failed http request");
                    },
                }
            });
            queue_pusher.push(fut).await;
        }
    };
    pin_mut!(connect_fut);
    futures::future::select(
        connect_fut,
        queue_poller
    ).await.factor_first().0
}

#[patch_fn(kind = "return",
           target = "Draw__13CIOWinManagerCFv" + 0x124)]
unsafe extern "C" fn hook_every_frame()
{
    if running_on_dolphin() {
        return
    }

    static mut COUNTER: u32 = 0;
    COUNTER += 1;
    if COUNTER == 4000 {
        COUNTER = 0;
        primeapi::printf(b"COUNTER reset\n\0".as_ptr());
    }

    let event_loop = if let Some(event_loop) = EVENT_LOOP.as_mut() {
        event_loop
    } else {
        return
    };
    let waker = empty_waker();
    let mut ctx = Context::from_waker(&waker);
    match event_loop.as_mut().poll(&mut ctx) {
        Poll::Pending => (),
        Poll::Ready(never) => match never { },
    }
}

// use alloc::rc::Rc;
// use alloc::vec::Vec;
// use core::cell::Cell;

// struct Reactor
// {
//     // TODO: SmallVec?
//     futures: Vec<(Pin<Box<dyn Future<Output = ()>>>, Rc<Cell<bool>>)>,
// }

// impl Reactor
// {
//     fn turn(&mut self)
//     {
//         static RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
//             // TODO: Should be Weak<Cell<bool>>, but the into_raw/from_raw apis are unstable
//             // clone
//             |p| {
//                 let rc = unsafe { Rc::from_raw(p as *const Cell<bool>) };
//                 Rc::into_raw(rc.clone());
//                 RawWaker::new(Rc::into_raw(rc) as *const (), &RAW_WAKER_VTABLE)
//             },
//             // wake
//             |p| {
//                 let rc = unsafe { Rc::from_raw(p as *const Cell<bool>) };
//                 rc.set(true);
//             },
//             // wake_by_ref
//             |p| {
//                 let rc = unsafe { Rc::from_raw(p as *const Cell<bool>) };
//                 rc.set(true);
//                 Rc::into_raw(rc);
//             },
//             // drop
//             |p| {
//                 unsafe { Rc::from_raw(p as *const Cell<bool>) };
//             }
//         );

//         for (fut, waker_flag) in self.futures.iter_mut() {
//             if !waker_flag.get() {
//                 continue
//             }

//             waker_flag.set(false);
//             let raw_waker = RawWaker::new(
//                 Rc::into_raw(waker_flag.clone()) as *const (),
//                 &RAW_WAKER_VTABLE
//             );
//             let waker = unsafe { Waker::from_raw(raw_waker) };
//             let mut context = Context::from_waker(&waker);

//             match fut.as_mut().poll(&mut context) {
//                 // TODO: How do I remove the future from the vec? Call this from retain
//                 Poll::Ready(()) => (),// TODO
//                 Poll::Pending => (),
//             }
//         }
//     }
// }

use alloc::vec::Vec;

use core::cmp;
use core::mem;
use core::ops::Range;
use core::marker::PhantomPinned;
// use primeapi::alignment_utils::{Aligned32, Aligned32Slice};
use primeapi::dol_sdk::dvd::AsyncDVDReadHandle;

use async_utils::MaybeUninitSliceExt;

use futures::ready;

use mini_http_server::{FileSystem, FileMetadata};

struct DvdFileSystem;
impl FileSystem for DvdFileSystem
{
    type File = DvdFile;
    fn open_file(&self, uri: &[u8]) -> Option<(DvdFile, FileMetadata)>
    {
        const MAX_FILENAME_LEN: usize = 128;

        if uri.len() >= MAX_FILENAME_LEN {
            return None;
        }

        // Ensure our filename is null-terminated
        let mut buf = [MaybeUninit::uninit(); MAX_FILENAME_LEN];
        buf[..uri.len()].copy_from_slice(<[MaybeUninit<_>]>::from_inited_slice(uri));
        buf[uri.len()] = MaybeUninit::new(0);
        let filename = unsafe { buf[..uri.len() + 1].assume_init_mut() };

        let fi = if let Some(fi) = DVDFileInfo::new(filename) {
            fi
        } else {
            return None;
        };
        let metadata = FileMetadata {
            size: fi.file_length(),
            // TODO: We should pull these values from a global variable/the config
            etag: None,
            last_modified: None,
        };
        let file = DvdFile {
            fi,
            pos: 0,
            read_state: DvdFileReadState::Empty,
            _pinned: PhantomPinned,
        };
        Some((file, metadata))

    }

}

struct DvdFile
{
    fi: DVDFileInfo,

    pos: u32,
    read_state: DvdFileReadState<'static>,

    _pinned: PhantomPinned,
}

enum DvdFileReadState<'a>
{
    InProgress(AsyncDVDReadHandle<'a, ()>, Box<[MaybeUninit<u8>]>),
    Filled(Range<usize>, Box<[MaybeUninit<u8>]>),
    Empty,
}

impl AsyncRead for DvdFile
{
    type Error = ();
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [MaybeUninit<u8>]
    ) -> Poll<Result<usize, Self::Error>>
    {
        let filled_buf = ready!(self.as_mut().poll_fill_buf(cx)?);

        let amt = cmp::min(buf.len(), filled_buf.len());
        let filled_buf = <[MaybeUninit<u8>]>::from_inited_slice(&filled_buf[..amt]);
        buf[..amt].copy_from_slice(filled_buf);
        self.consume(amt);
        Poll::Ready(Ok(amt))
    }
}

impl AsyncBufRead for DvdFile
{
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<Result<&[u8], Self::Error>>
    {
        let this = unsafe { self.get_unchecked_mut() };

        match mem::replace(&mut this.read_state, DvdFileReadState::Empty) {
            DvdFileReadState::InProgress(op, buffer) if op.is_finished() => {
                // TODO: Check error?
                let real_bytes_read = cmp::min(
                    (this.fi.file_length() - this.pos) as usize,
                    buffer.len()
                );
                this.pos += real_bytes_read as u32;
                this.read_state = DvdFileReadState::Filled(0..real_bytes_read, buffer);
                unsafe { Pin::new_unchecked(this) }.poll_fill_buf(cx)
            },
            DvdFileReadState::InProgress(op, buffer) => {
                this.read_state = DvdFileReadState::InProgress(op, buffer);
                Poll::Pending
            },

            DvdFileReadState::Filled(valid_range, buffer) => {
                this.read_state = DvdFileReadState::Filled(valid_range.clone(), buffer);
                let buffer = match &mut this.read_state {
                    DvdFileReadState::Filled(_, buffer) => buffer,
                    _ => unreachable!(),
                };
                Poll::Ready(Ok(unsafe { &buffer[valid_range.clone()].assume_init() }))
            },
            DvdFileReadState::Empty => {
                if this.pos == this.fi.file_length() {
                    return Poll::Ready(Ok(&[]))
                }

                // XXX We're taking advantage of the fact that MP1's malloc always returns a
                //     32-byte aligned pointer.
                let l = cmp::min(4096, (this.fi.file_length() - this.pos + 31) & !31) as usize;
                let mut buf = unsafe {
                    Vec::from_raw_parts(primeapi::malloc(l) as *mut MaybeUninit<u8>, l, l)
                        .into_boxed_slice()
                };
                let op = unsafe {
                    // TODO: Use the callback version to do the waker thang
                    mem::transmute(this.fi.read_async(
                        Aligned32SliceMut::from_slice_unchecked(&mut buf[..]),
                        0,
                        0
                    ))
                };
                this.read_state = DvdFileReadState::InProgress(op, buf);
                Poll::Pending
            },
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize)
    {
        let this = unsafe { self.get_unchecked_mut() };

        match &mut this.read_state {
            DvdFileReadState::InProgress(_, _) => (), // XXX panic?
            DvdFileReadState::Empty => (),
            DvdFileReadState::Filled(valid_range, _buffer) => {
                valid_range.start += amt;
                if valid_range.start >= valid_range.end {
                    this.read_state = DvdFileReadState::Empty;
                }
            },
        }
    }
}

use primeapi::dol_sdk::os::{OSGetTime, Ticks};

pub fn delay(ticks: Ticks) -> impl Future<Output = ()>
{
    let finished = ticks.ticks() + OSGetTime();
    async_utils::poll_until(move || OSGetTime() >= finished)
}

use embedded_websocket::{
    Error as WebSocketError, WebSocketKey, WebSocketReceiveMessageType, WebSocketSendMessageType,
    WebSocketServer, WebSocketState,
};
use futures::future::{self, TryFutureExt};

use async_utils::io::{AsyncWrite, AsyncReadExt};
use core::cell::RefCell;

use crate::tracker_state::{TrackerState, update_tracker_state};

async fn websocket_logic<W, R>(
    mut reader: R,
    mut writer: W,
    ws_key: [u8; 24]
) -> Result<(), ()>
    where R: AsyncRead + Unpin,
          W: AsyncWrite + Unpin,
          W::Error: core::fmt::Debug + async_utils::io::AsyncIoError,
{
    let mut ws = WebSocketServer::new_server();
    let res: Result<(), ()> = async {
        let key_str = core::str::from_utf8(&ws_key[..])
            .map_err(|_| ())?;
        let key = <WebSocketKey as core::str::FromStr>::from_str(key_str)
            .map_err(|_| ())?;

        let mut buf = Box::new([0; 128]);
        let written = ws.server_accept(&key, None, &mut buf[..])
            .map_err(|_| ())?;
        writer.write_all(&buf[..written]).await
            .map_err(|_| ())?;
        Ok(())
    }.await;
    if res.is_err() {
        return Ok(())
    }

    let mut msg_buf = Box::new([0; 768]);
    let ws = RefCell::new(ws);
    let write_queue = async_utils::AsyncMsgQueue::new();

    let msg_fut = async {
        let l = msg_buf.len();
        let (msg_encoded, msg_decoded) = msg_buf.split_at_mut(l / 2 + 1);

        let mut ts = TrackerState::new();
        if let Some(len) = update_tracker_state(&mut ts, true, false, msg_decoded) {
            let len = len.get();
            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, &msg_decoded[..len], msg_encoded);
            match res {
                Ok(i) => {
                    // primeapi::dbg!(&msg_encoded[..i]);
                    write_queue.sync_push(&msg_encoded[..i]).await
                },
                Err(e) => {
                    primeapi::dbg!(e);
                },
            }
        };

        let interval = Ticks::from_millis(5000);
        let mut next_full_update = OSGetTime() + interval.ticks();
        loop {
            crate::delay(Ticks::from_millis(2500)).await;

            let curr_time = OSGetTime();
            let full_update = curr_time > next_full_update;
            if full_update {
                next_full_update = curr_time + interval.ticks();
            }

            let len = if let Some(len) = update_tracker_state(&mut ts, false, full_update, msg_decoded) {
                len.get()
            } else {
                continue
            };

            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, &msg_decoded[..len], msg_encoded);
            match res {
                Ok(i) => {
                    // primeapi::dbg!(&msg_encoded[..i]);
                    write_queue.sync_push(&msg_encoded[..i]).await
                },
                Err(e) => {
                    primeapi::dbg!(e);
                },
            }
        };
    };

    let ping_fut = async {
        let mut ping_buf = Box::new([0; 64]);
        // Every ~10 seconds send a ping
        loop {
            crate::delay(Ticks::from_millis(10000)).await;
            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Ping, true, &[], &mut ping_buf[..]);
            match res {
                Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                Err(e) => { primeapi::dbg!(e); },
            }
            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, b"{\"ping\":null}", &mut ping_buf[..]);
            match res {
                Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                Err(e) => { primeapi::dbg!(e); },
            }
        }
    };

    let reader_fut = async {
        let mut recv_encoded = Box::new([0; 512 + 32]);
        let mut recv_decoded = Box::new([0; 512]);
        let mut recv_encoded_len = 0;
        let r = loop {
            if ws.borrow().state != WebSocketState::Open {
                break Ok(())
            }

            let res = ws
                .borrow_mut()
                .read(&recv_encoded[..recv_encoded_len], &mut recv_decoded[..]);
            let res = match res {
                Ok(res) => res,
                Err(WebSocketError::ReadFrameIncomplete) => {
                    let buf = <[MaybeUninit<u8>]>::from_inited_slice_mut(&mut recv_encoded[..]);
                    recv_encoded_len += reader.read(buf).await.map_err(|_| ())?;
                    continue
                },
                Err(e) => {
                    primeapi::dbg!(e);
                    break Err(());
                },
            };

            recv_encoded_len -= res.len_from;
            unsafe {
                ptr::copy(
                    recv_encoded[res.len_from..].as_ptr(),
                    recv_encoded.as_mut_ptr(),
                    res.len_from,
                );
            }

            if res.message_type == WebSocketReceiveMessageType::Ping {
                // Send a pong
                let l = ws.borrow_mut()
                    .write(WebSocketSendMessageType::Pong, true, &[], &mut recv_decoded[..])
                    .map_err(|e| { primeapi::dbg!(e); })?;
                    // .map_err(|_| ())?;
                write_queue.sync_push(&recv_decoded[..l]).await;
            } else if res.message_type == WebSocketReceiveMessageType::CloseMustReply {
                let l = ws.borrow_mut()
                    .write(WebSocketSendMessageType::CloseReply, true, &[], &mut recv_decoded[..])
                    .map_err(|e| { primeapi::dbg!(e); })?;
                    // .map_err(|_| ())?;
                write_queue.sync_push(&recv_decoded[..l]).await;
            } else if res.message_type == WebSocketReceiveMessageType::CloseCompleted {
                return Err(())
            }
        };
        r
    };

    let write_fut = async {
        loop {
            if false {
                // XXX Type hint
                break Result::<(), W::Error>::Ok(())
            }
            let buf_ref = write_queue.sync_pop().await;
            writer.write_all(&buf_ref).await?;
        }
    };

    pin_mut!(msg_fut, reader_fut, ping_fut, write_fut);
    let f = future::select(
        write_fut.map_err(|e| { primeapi::dbg!(e); }),
        reader_fut
    );
    let f = future::select(f, msg_fut);
    let f = future::select(f, ping_fut);
    let _ = f.await;
    Ok(())
}
