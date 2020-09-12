use alloc::boxed::Box;
use alloc::vec::Vec;

use core::cmp;
use core::cell::RefCell;
use core::future::Future;
use core::mem;
use core::ops::Range;
use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

use embedded_websocket::{
    Error as WebSocketError, WebSocketKey, WebSocketReceiveMessageType, WebSocketSendMessageType,
    WebSocketServer, WebSocketState,
};
use futures::future::{self, TryFutureExt};
use futures::ready;
use futures::never::Never;
use pin_utils::pin_mut;

use async_utils::MaybeUninitSliceExt;
use async_utils::io::{AsyncRead, AsyncBufRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use mini_http_server::{FileSystem, FileMetadata};

use primeapi::alignment_utils::Aligned32;
use primeapi::dol_sdk::dvd::{AsyncDVDReadHandle, DVDFileInfo};
use primeapi::dol_sdk::os::{OSGetTime, Ticks};

use crate::nintendont_sock::SocketApi;
use crate::tracker_state::{TrackerState, update_tracker_state};


pub(crate) async fn event_loop() -> Never
{
    let mut server = SocketApi::global_instance().tcp_server(80, 1).unwrap();
    let mut queue = async_utils::FutureQueue::<_, generic_array::typenum::U5>::new();
    let (queue_poller, mut queue_pusher) = queue.split();
    let connect_fut = async {
        loop {
            // Implement some kind of timeout for http requests
            let (mut client, _addr) = server.accept().await.unwrap();

            primeapi::dbg!("Accepted connection");
            let fut = Box::pin(async move {
                let (mut reader, mut writer) = client.split();
                let handler = mini_http_server::HttpRequestHandler::new()
                    .reader(&mut reader)
                    .writer(&mut writer)
                    .fs(DvdFileSystem)
                    .ws(|_uri, key, r, w| {
                        Ok(websocket_logic(r, w, key))
                    });

                match handler.handle_http_request().await {
                    Ok(b) => {
                        primeapi::dbg!("successful http request", b);
                    },
                    Err(mini_http_server::HttpRequestError::ReaderIO(_e)) => {
                        primeapi::dbg!("Reader IO Error", _e);
                    },
                    Err(mini_http_server::HttpRequestError::WriterIO(_e)) => {
                        primeapi::dbg!("Writer IO Error", _e);
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
                        Aligned32::from_mut_unchecked(&mut buf[..]),
                        this.pos,
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

fn delay(ticks: Ticks) -> impl Future<Output = ()>
{
    let finished = ticks.ticks() + OSGetTime();
    async_utils::poll_until(move || OSGetTime() >= finished)
}

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
                    write_queue.sync_push(&msg_encoded[..i]).await
                },
                Err(_e) => {
                    primeapi::dbg!(_e);
                },
            }
        };

        let interval = Ticks::from_millis(5000);
        let mut next_full_update = OSGetTime() + interval.ticks();
        loop {
            delay(Ticks::from_millis(2500)).await;

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
                    write_queue.sync_push(&msg_encoded[..i]).await
                },
                Err(_e) => {
                    primeapi::dbg!(_e);
                },
            }
        };
    };

    let ping_fut = async {
        let mut ping_buf = Box::new([0; 64]);
        // Every ~10 seconds send a ping
        loop {
            delay(Ticks::from_millis(10000)).await;
            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Ping, true, &[], &mut ping_buf[..]);
            match res {
                Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                Err(_e) => { primeapi::dbg!(_e); },
            }
            let res = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, b"{\"ping\":null}", &mut ping_buf[..]);
            match res {
                Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                Err(_e) => { primeapi::dbg!(_e); },
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
                Err(_e) => {
                    primeapi::dbg!(_e);
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
                    .map_err(|_e| { primeapi::dbg!(_e); })?;
                    // .map_err(|_| ())?;
                write_queue.sync_push(&recv_decoded[..l]).await;
            } else if res.message_type == WebSocketReceiveMessageType::CloseMustReply {
                let l = ws.borrow_mut()
                    .write(WebSocketSendMessageType::CloseReply, true, &[], &mut recv_decoded[..])
                    .map_err(|_e| { primeapi::dbg!(_e); })?;
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
        write_fut.map_err(|_e| { primeapi::dbg!(_e); }),
        reader_fut
    );
    let f = future::select(f, msg_fut);
    let f = future::select(f, ping_fut);
    let _ = f.await;
    Ok(())
}
