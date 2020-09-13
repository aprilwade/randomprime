use alloc::boxed::Box;
use alloc::vec::Vec;

use core::cmp;
use core::cell::RefCell;
use core::convert::Infallible;
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
use futures::future::{self, FutureExt};
use futures::ready;
use futures::never::Never;
use pin_utils::pin_mut;

use async_utils::{async_type_hint, MaybeUninitSliceExt};
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
                loop {
                    let handler = mini_http_server::HttpRequestHandler::new()
                        .reader(&mut reader)
                        .writer(&mut writer)
                        .fs(DvdFileSystem)
                        .ws(|_uri, key, r, w| {
                            Ok(websocket_logic(r, w, key))
                        });

                    primeapi::dbg!("Handing request");
                    // TODO: Place some kind of timeout on a non-websocket http request
                    match handler.handle_http_request().await {
                        // If we got a true, we can reuse this connection.
                        Ok(true) => { primeapi::dbg!("successful http request - reusing"); },
                        Ok(false) => {
                            primeapi::dbg!("successful http request - closing");
                            break
                        },
                        Err(mini_http_server::HttpRequestError::ReaderIO(e)) => {
                            primeapi::dbg!("Reader IO Error", e);
                            break
                        },
                        Err(mini_http_server::HttpRequestError::WriterIO(e)) => {
                            primeapi::dbg!("Writer IO Error", e);
                            break
                        },
                        Err(mini_http_server::HttpRequestError::Internal) => {
                            primeapi::dbg!("failed http request");
                            break
                        },
                    }

                    // Wait until either we have additional data to read (indicating another
                    // requests over the persistent connection), or a timeout expires.
                    let res = future::select(
                        reader.wait_until_read_available(),
                        delay(Ticks::from_seconds(5)),
                    ).await;
                    match res {
                        future::Either::Left((Ok(()), _)) => {
                            primeapi::dbg!("Data available to read");
                        },
                        future::Either::Left((Err(e), _)) => {
                            primeapi::dbg!("Reader IO Error", e);
                            break
                        }
                        future::Either::Right(((), _)) => {
                            primeapi::dbg!("keep-alive timeout");
                            break
                        },
                    }
                }
            });
            queue_pusher.push(fut).await;
        }
    };
    pin_mut!(connect_fut);
    future::select(
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

        let prefix = &b"/tracker"[..];
        let suffix = if let Some(b'/') = uri.last() {
            &b"index.html"[..]
        } else {
            &b""[..]
        };

        if prefix.len() + uri.len() + suffix.len() >= MAX_FILENAME_LEN {
            return None;
        }

        let mut buf = [MaybeUninit::uninit(); MAX_FILENAME_LEN];
        let mut idx = 0;
        for slice in &[prefix, uri, suffix] {
            buf[idx..idx + slice.len()].copy_from_slice(<[MaybeUninit<_>]>::from_inited_slice(slice));
            idx += slice.len();
        }
        // TODO: Perform percent/url decode in-place

        // Ensure the filename is null-terminated
        buf[idx] = MaybeUninit::new(0);

        let filename = unsafe { buf[..idx + 1].assume_init_mut() };
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
          R::Error: core::fmt::Debug,
          W: AsyncWrite + Unpin,
          W::Error: core::fmt::Debug + async_utils::io::AsyncIoError,
{
    let mut ws = WebSocketServer::new_server();
    let res: Result<(), ()> = async {
        let key_str = core::str::from_utf8(&ws_key[..])
            .map_err(|e| { primeapi::dbg!(e); })?;
        let key = <WebSocketKey as core::str::FromStr>::from_str(key_str)
            .map_err(|e| { primeapi::dbg!(e); })?;

        let mut buf = Box::new([0; 128]);
        let written = ws.server_accept(&key, None, &mut buf[..])
            .map_err(|e| { primeapi::dbg!(e); })?;
        writer.write_all(&buf[..written]).await
            .map_err(|e| { primeapi::dbg!(e); })?;
        Ok(())
    }.await;
    if res.is_err() {
        return Ok(())
    }

    let mut msg_buf = Box::new([0; 768]);
    let ws = RefCell::new(ws);
    let write_queue = async_utils::AsyncMsgQueue::new();

    let msg_fut = async {
        async_type_hint!(Result<Infallible, ()>);
        let l = msg_buf.len();
        let (msg_encoded, msg_decoded) = msg_buf.split_at_mut(l / 2 + 1);

        let mut ts = TrackerState::new();
        if let Some(len) = update_tracker_state(&mut ts, true, false, msg_decoded) {
            let len = len.get();
            let written_len = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, &msg_decoded[..len], msg_encoded)
                .map_err(|e| { primeapi::dbg!(e); })?;
            write_queue.sync_push(&msg_encoded[..written_len]).await
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

            let written_len = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, &msg_decoded[..len], msg_encoded)
                .map_err(|e| { primeapi::dbg!(e); })?;
            write_queue.sync_push(&msg_encoded[..written_len]).await;
        };
    };

    let ping_fut = async {
        async_type_hint!(Result<Infallible, ()>);

        let mut ping_buf = Box::new([0; 64]);
        // Every ~10 seconds send a ping
        loop {
            delay(Ticks::from_millis(10000)).await;
            let written_len = ws.borrow_mut()
                .write(WebSocketSendMessageType::Ping, true, &[], &mut ping_buf[..])
                .map_err(|e| { primeapi::dbg!(e); })?;
            write_queue.sync_push(&ping_buf[..written_len]).await;

            let written_len = ws.borrow_mut()
                .write(WebSocketSendMessageType::Text, true, b"{\"ping\":null}", &mut ping_buf[..])
                .map_err(|e| { primeapi::dbg!(e); })?;
            write_queue.sync_push(&ping_buf[..written_len]).await;
        }
    };

    let reader_fut = async {
        let mut recv_encoded = Box::new([0; 512 + 32]);
        let mut recv_decoded = Box::new([0; 512]);
        let mut recv_encoded_len = 0;
        loop {
            if ws.borrow().state != WebSocketState::Open {
                // TODO: Why is this Ok instead of Err?
                return Ok(())
            }

            let res = ws
                .borrow_mut()
                .read(&recv_encoded[..recv_encoded_len], &mut recv_decoded[..]);
            let res = match res {
                Ok(res) => res,
                Err(WebSocketError::ReadFrameIncomplete) => {
                    let buf = <[MaybeUninit<u8>]>::from_inited_slice_mut(&mut recv_encoded[..]);
                    let f = future::select(
                        reader.read(buf),
                        delay(Ticks::from_seconds(30)),
                    );
                    recv_encoded_len += match f.await {
                        // Recving a 0 means the other side has shutdown their side of the
                        // connection and can no longer send additional data. Since we send pings
                        // regularly and expect pongs back, we take this as a sign the websocket is
                        // toast.
                        future::Either::Left((Ok(0), _)) => return Err(()),
                        future::Either::Left((Ok(i), _)) => i,
                        future::Either::Left((Err(e), _)) => {
                            primeapi::dbg!(e);
                            return Err(())
                        },
                        future::Either::Right(((), _)) => {
                            primeapi::dbg!("Timeout waiting for WS read");
                            return Err(())
                        },
                    };
                    continue
                },
                Err(e) => {
                    primeapi::dbg!(e);
                    return Err(());
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

            primeapi::dbg!(res.message_type);
            if res.message_type == WebSocketReceiveMessageType::Ping {
                // Send a pong
                let l = ws.borrow_mut()
                    .write(WebSocketSendMessageType::Pong, true, &[], &mut recv_decoded[..])
                    .map_err(|e| { primeapi::dbg!(e); })?;
                write_queue.sync_push(&recv_decoded[..l]).await;
            } else if res.message_type == WebSocketReceiveMessageType::CloseMustReply {
                let l = ws.borrow_mut()
                    .write(WebSocketSendMessageType::CloseReply, true, &[], &mut recv_decoded[..])
                    .map_err(|e| { primeapi::dbg!(e); })?;
                write_queue.sync_push(&recv_decoded[..l]).await;
            } else if res.message_type == WebSocketReceiveMessageType::CloseCompleted {
                return Err(())
            }
        }
    };

    let write_fut = async {
        // async_type_hint!(Result<Infallible, ()>);
        loop {
            let buf_ref = write_queue.sync_pop().await;
            let res = future::select(
                writer.write_all(&buf_ref),
                delay(Ticks::from_seconds(30)),
            ).await;
            match res {
                future::Either::Left((Ok(i), _)) => i,
                future::Either::Left((Err(e), _)) => {
                    primeapi::dbg!(e);
                    return;
                },
                future::Either::Right(((), _)) => {
                    primeapi::dbg!("Timeout expired trying to write to WS");
                    return;
                },

            }
        }
    };

    pin_mut!(msg_fut, reader_fut, ping_fut, write_fut);
    let () = futures::select_biased! {
        _ = write_fut.fuse() => (),
        _ = reader_fut.fuse() => (),
        _ = msg_fut.fuse() => (),
        _ = ping_fut.fuse() => (),
    };
    Ok(())
}
