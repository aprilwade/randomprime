
extern crate alloc;

use arrayvec::ArrayString;
use embedded_websocket::{
    Error as WebSocketError, WebSocketKey, WebSocketReceiveMessageType, WebSocketSendMessageType,
    WebSocketServer, WebSocketState,
};
use futures::future::{self, TryFutureExt};
use futures::never::Never;
use pin_utils::pin_mut;

use alloc::borrow::{Cow, ToOwned};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::default::Default;
use core::fmt::Write;
use core::future::Future;
use core::mem::MaybeUninit;
use core::ptr;
use core::writeln;

use primeapi::alignment_utils::{Aligned32, Aligned32SliceMut};

use async_utils::{
    async_write_all, AsyncRead, AsyncWrite, BufferedAsyncWriter, LineReader, LineReaderError,
    MaybeUninitSliceExt,
};


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum HttpMethod
{
    Get,
    Head,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum HttpStatus
{
    OK200,
    NotModified304,
    BadRequest400,
    NotFound404,
    MethodNotAllowed405,
    RequestTimeout408,
    UriTooLong414,
    TooManyRequests429,
    RequestHeaderFieldsTooLarge431,
    InternalServerError500,
    NotImplemented501,
    HttpVersionNotSupported505,
    ServiceUnavailable503,
}

impl HttpStatus
{
    pub fn header_message(&self) -> &'static str
    {
        match self {
            HttpStatus::OK200 => "200 OK\r\n",
            HttpStatus::NotModified304 => "304 Not Modified\r\n",
            HttpStatus::BadRequest400 => "400 Bad Request\r\n",
            HttpStatus::NotFound404 => "404 Not Found\r\n",
            HttpStatus::MethodNotAllowed405 => "405 Method Not Allowed\r\n",
            HttpStatus::RequestTimeout408 => "408 Request Timeout\r\n",
            HttpStatus::UriTooLong414 => "414 URI Too Long\r\n",
            HttpStatus::TooManyRequests429 => "429 Too Many Requests\r\n",
            HttpStatus::RequestHeaderFieldsTooLarge431 => "431 Request Header Fields Too Large\r\n",
            HttpStatus::InternalServerError500 => "500 Internal Server Error\r\n",
            HttpStatus::NotImplemented501 => "501 Not Implemented\r\n",
            HttpStatus::HttpVersionNotSupported505 => "505 HTTP Version Not Supported\r\n",
            HttpStatus::ServiceUnavailable503 => "503 Service Unavailable\r\n",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HttpSemanticError(pub(crate) HttpStatus, pub(crate) Cow<'static, str>);

#[derive(Clone, Debug)]
pub(crate) enum HttpError<RE, WE>
{
    Semantic(HttpSemanticError),
    ReaderIO(RE),
    WriterIO(WE),
    Unrecoverable,
}

impl<RE, WE> HttpError<RE, WE>
{
    pub fn semantic<S: Into<Cow<'static, str>>>(status: HttpStatus, s: S) -> Self
    {
        HttpError::Semantic(HttpSemanticError(status, s.into()))
    }
}

impl<RE, WE> From<LineReaderError<RE>> for HttpError<RE, WE>
{
    fn from(e: LineReaderError<RE>) -> Self
    {
        match e {
            LineReaderError::Inner(e) => HttpError::ReaderIO(e),
            LineReaderError::MaxLengthExceeded => HttpError::semantic(
                HttpStatus::RequestHeaderFieldsTooLarge431,
                ""
            ),
        }
    }
}

pub(crate) async fn parse_http_request_headers<R, B, WE>(
    lr: &mut LineReader<R, B>,
    handler: &mut SupportedHttpHeaderFields,
) -> Result<(), HttpError<R::Error, WE>>
    where R: AsyncRead,
          B: alloc::borrow::BorrowMut<[MaybeUninit<u8>]>,
{
    let l = lr.read_line().await?;
    let l = if l.last() == Some(&b'\r') { &l[..l.len() - 1] } else { l };
    let mut parts = l.split(|b| *b == b' ').filter(|p| p.len() > 0);
    let method_name = parts.next()
        .ok_or_else(|| HttpError::semantic(HttpStatus::BadRequest400, ""))?;
    let method = match method_name {
        b"GET" => HttpMethod::Get,
        b"HEAD" => HttpMethod::Head,
        _ => Err(HttpError::semantic(HttpStatus::NotImplemented501, ""))?,
    };
    let uri = parts.next()
        .ok_or_else(|| HttpError::semantic(HttpStatus::BadRequest400, ""))?;
    let http_version = parts.next()
        .ok_or_else(|| HttpError::semantic(HttpStatus::BadRequest400, ""))?;
    if parts.next() != None {
        Err(HttpError::semantic(HttpStatus::BadRequest400, ""))?;
    }
    if http_version != b"HTTP/1.1" {
        Err(HttpError::semantic(HttpStatus::HttpVersionNotSupported505, ""))?;
    }
    handler.accept_method(method, uri).map_err(HttpError::Semantic)?;

    loop {
        let l = lr.read_line().await?;
        let l = if l.last() == Some(&b'\r') { &l[..l.len() - 1] } else { l };
        if l.len() == 0 {
            break
        }

        let colon_pos = l.iter()
            .position(|b| *b == b':')
            .ok_or_else(|| HttpError::semantic(HttpStatus::BadRequest400, ""))?;
        let (field_name, rest) = l.split_at(colon_pos);
        let rest = &rest[1..];// Remove the colon
        let whitespace_len = rest.iter()
            .position(|b| *b != b' ')
            .unwrap_or(rest.len());
        handler.accept_header_field(field_name, &rest[whitespace_len..])
            .map_err(HttpError::Semantic)?;
    }

    Ok(())
}

#[derive(Debug, Default)]
pub(crate) struct SupportedHttpHeaderFields
{
    pub(crate) method: Option<HttpMethod>,
    pub(crate) uri: Vec<u8>,
    pub(crate) content_length: Option<usize>,
    pub(crate) if_modified_since: Option<HttpDate>,
    pub(crate) if_none_match: Option<[u8; 16]>,
    // TODO If we receive "Connection: close", then we should terminate the connection after
    //      sending our response

    pub(crate) connection_upgrade: bool,
    pub(crate) upgrade_websocket: bool,
    pub(crate) websocket_key: Option<[u8; 24]>,
    pub(crate) websocket_version_13: bool,
}

impl SupportedHttpHeaderFields
{
    fn accept_method(&mut self, method: HttpMethod, uri: &[u8]) -> Result<(), HttpSemanticError>
    {
        self.method = Some(method);
        self.uri = uri.to_owned();
        Ok(())
    }

    fn accept_header_field(&mut self, name: &[u8], val: &[u8]) -> Result<(), HttpSemanticError>
    {
        if name == b"Content-Length" {
            let len = core::str::from_utf8(val)
                .map_err(|_| HttpSemanticError(HttpStatus::BadRequest400,
                                               "Invalid Content-Length".into()))?
                .parse()
                .map_err(|_| HttpSemanticError(HttpStatus::BadRequest400,
                                               "Invalid Content-Length".into()))?;
            self.content_length = Some(len);
        } else if name == b"If-Modified-Since" {
            let s = core::str::from_utf8(val)
                .map_err(|_| HttpSemanticError(
                        HttpStatus::BadRequest400,
                        "Invalid If-Modified-Since".into()
                    ))?;
            self.if_modified_since = HttpDate::from_str(s);
        } else if name == b"If-None-Match" {
            let s = core::str::from_utf8(val)
                .map_err(|_| HttpSemanticError(
                        HttpStatus::BadRequest400,
                        "Invalid ETag".into()
                    ))?;
            let s = s.trim().trim_matches('"');
            if s.len() <= 16 {
                let mut etag = [0; 16];
                etag[..s.len()].copy_from_slice(s.as_bytes());
                self.if_none_match = Some(etag);
            }
        } else if name == b"Connection" {
            let s = core::str::from_utf8(val)
                .map_err(|_| HttpSemanticError(
                        HttpStatus::BadRequest400,
                        "Invalid Connection value".into()
                    ))?;
            for s in s.split(',') {
                if s.trim() == "Upgrade" {
                    self.connection_upgrade = true;
                }
            }
        } else if name == b"Upgrade" && val == b"websocket" {
            self.upgrade_websocket = true;
        } else if name == b"Sec-WebSocket-Key" {
            let mut key_buf = [0; 24];
            if val.len() > key_buf.len() {
                Err(HttpSemanticError(
                    HttpStatus::BadRequest400,
                    "Invalid length for Sec-WebSocket-Key".into()
                ))?
            }
            key_buf[..val.len()].copy_from_slice(val);
            self.websocket_key = Some(key_buf);
        } else if name == b"Sec-WebSocket-Version" {
            let ver_num: u32 = core::str::from_utf8(val)
                .map_err(|_| HttpSemanticError(HttpStatus::BadRequest400,
                                               "Invalid Sec-WebSocket-Version".into()))?
                .parse()
                .map_err(|_| HttpSemanticError(HttpStatus::BadRequest400,
                                               "Invalid Sec-WebSocket-Version".into()))?;
            if ver_num != 13 {
                Err(HttpSemanticError(
                    HttpStatus::BadRequest400,
                    "Invalid Sec-WebSocket-Version".into()
                ))?
            }
            self.websocket_version_13 = true;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct HttpDate(u32);

impl HttpDate
{
    pub fn from_parts(
        year: u16, month: u8, day: u8,
        hour: u8, minute: u8, second: u8,
    ) -> HttpDate
    {
        let mut i = if year < 2000 { 0 } else { (year - 2000) as u32 };
        i = i << 4 | (month - 1) as u32;
        i = i << 5 | (day - 1) as u32;
        i = i << 5 | hour as u32;
        i = i << 6 | minute as u32;
        i = i << 6 | second as u32;
        HttpDate(i)
    }

    pub fn from_str(s: &str) -> Option<HttpDate>
    {
        // TODO: Technically, RFC 1123 is a fixed length format, so we could make this
        //       much simpler & more compact
        //       https://tools.ietf.org/html/rfc2616#section-3.3.1
        let res: Result<HttpDate, ()> = (|| {
            let mut parts = s.split(',');
            let _ = parts.next()
                .ok_or_else(|| ())?;
            let rest = parts.next()
                .ok_or_else(|| ())?;

            if parts.next().is_some() {
                Err(())?;
            }

            let mut parts = rest.trim().split(' ').filter(|p| p.len() > 0);

            let day = parts.next()
                .ok_or_else(|| ())?
                .parse()
                .map_err(|_| ())?;
            let month = match parts.next().ok_or_else(|| ())? {
                "Jan" => 1, "Feb" =>  2, "Mar" =>  3, "Apr" =>  4,
                "May" => 5, "Jun" =>  6, "Jul" =>  7, "Aug" =>  8,
                "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
                _ => Err(())?,
            };
            let year = parts.next()
                .ok_or_else(|| ())?
                .parse()
                .map_err(|_| ())?;
            let hms = parts.next()
                .ok_or_else(|| ())?;

            if parts.next() != Some("GMT") {
                Err(())?;
            }
            if parts.next().is_some() {
                Err(())?;
            }

            let mut hms_parts = hms.split(':');

            let hour = hms_parts.next()
                .ok_or_else(|| ())?
                .parse()
                .map_err(|_| ())?;
            let minute = hms_parts.next()
                .ok_or_else(|| ())?
                .parse()
                .map_err(|_| ())?;
            let second = hms_parts.next()
                .ok_or_else(|| ())?
                .parse()
                .map_err(|_| ())?;

            Ok(HttpDate::from_parts(year, month, day, hour, minute, second))
        })();
        res.ok()
    }
}

pub struct FileMetadata
{
    pub size: u32,
    pub last_modified: Option<&'static str>,
    pub etag: Option<[u8; 16]>,
}

struct DvdFileSystem;
impl DvdFileSystem
{
    fn lookup_file(&self, uri: &[u8]) -> Option<(FileMetadata, DvdFileSystemReader)>
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
        Some((metadata, DvdFileSystemReader(fi, 0)))
    }
}

use primeapi::dol_sdk::dvd::DVDFileInfo;
struct DvdFileSystemReader(DVDFileInfo, u32);
impl DvdFileSystemReader
{
    async fn async_read(&mut self, mut buf: &mut [MaybeUninit<u8>]) -> Result<usize, Never>
    {
        if self.1 >= self.0.file_length() {
            return Ok(0)
        }
        let bytes_read = if self.1 & 3 != 0 {
            // The offset into the file must be a multiple of 4, so perform an extra small read to
            // fix the alignment if, needed
            let mut tmp_buf = Aligned32::new([MaybeUninit::uninit(); 32]);
            {
                let handler = self.0.read_async(tmp_buf.as_inner_slice_mut(), self.1 & 3, 0);
                async_utils::poll_until(|| handler.is_finished()).await;
            }
            let bytes_to_copy = core::cmp::min(32 - self.1 as usize & 3, buf.len());
            buf[..bytes_to_copy].copy_from_slice(&tmp_buf[..bytes_to_copy]);
            buf = &mut buf[bytes_to_copy..];
            self.1 += bytes_to_copy as u32;
            bytes_to_copy
        } else {
            0
        };

        let buf_addr = buf.as_ptr() as usize;
        if buf.len() < 31 || ((buf_addr + 31) & !31) >= ((buf_addr + buf.len()) & !31) {
            if buf.len() == 0 {
                return Ok(bytes_read)
            }
            // The number of bytes read from the disc must be a multiple of 32. Normally we
            // accomplish this by simply truncating the read length to the nearest multiple of 32,
            // but if we're being asked to copy less than 32 bytes, we need to copy the data into a
            // temporary buffer first to avoid any chance of overrunning the provided buffer
            let mut tmp_buf = Aligned32::new([MaybeUninit::uninit(); 32]);
            {
                let handler = self.0.read_async(tmp_buf.as_inner_slice_mut(), self.1, 0);
                async_utils::poll_until(|| handler.is_finished()).await;
            }
            let len = core::cmp::min(buf.len(), 32);
            buf.copy_from_slice(&tmp_buf[..len]);

            return Ok(bytes_read + len)
        }

        let (unaligned_buf, mut aligned_buf) = Aligned32SliceMut::split_unaligned_prefix(buf);
        let file_remainder = (self.0.file_length() - self.1) as usize;
        let read_len = core::cmp::min(aligned_buf.len() & !31, (file_remainder + 31) & !31);
        let copy_len = core::cmp::min(read_len, file_remainder);
        {
            let recv_buf = aligned_buf.reborrow().truncate_to_len(read_len);
            let handler = self.0.read_async(recv_buf, self.1, 3);
            async_utils::poll_until(|| handler.is_finished()).await;
        }
        unsafe {
            ptr::copy(
                aligned_buf.as_ptr(),
                unaligned_buf.as_mut_ptr(),
                copy_len,
            );
        }
        self.1 += copy_len as u32;
        Ok(bytes_read + copy_len)
    }
}


struct WebSocketHandler;

impl WebSocketHandler
{
    fn websocket_connection<'a, W, R>(
        &mut self,
        _path: &[u8],
        ws_key: [u8; 24],
        reader: R,
        writer: W,
        buf: &'a mut [MaybeUninit<u8>],
    ) -> Result<impl Future<Output = ()> + 'a, W>
        where R: AsyncRead + 'a,
              W: AsyncWrite + 'a,
              W::Error: core::fmt::Debug,
    {
        Ok(Self::websocket_logic(reader, writer, buf, ws_key))
    }

    async fn websocket_logic<W, R>(
        mut reader: R,
        mut writer: W,
        buf: &mut [MaybeUninit<u8>],
        ws_key: [u8; 24]
    )
        where R: AsyncRead,
              W: AsyncWrite,
              W::Error: core::fmt::Debug,
    {
        let buf = unsafe {
            core::ptr::write_bytes(buf.as_mut_ptr(), 0, buf.len());
            buf.assume_init_mut()
        };
        let mut ws = WebSocketServer::new_server();
        let res: Result<(), ()> = async {
            let key_str = core::str::from_utf8(&ws_key[..])
                .map_err(|_| ())?;
            let key = <WebSocketKey as core::str::FromStr>::from_str(key_str)
                .map_err(|_| ())?;

            let written = ws.server_accept(&key, None, buf)
                .map_err(|_| ())?;
            async_utils::async_write_all(&mut writer, &buf[..written]).await
                .map_err(|_| ())?;
            Ok(())
        }.await;
        if res.is_err() {
            return
        }

        let (msg_buf, buf) = { buf }.split_at_mut(1024);
        let (ping_buf, recv_buf) = { buf }.split_at_mut(64);
        let ws = RefCell::new(ws);
        let write_queue = async_utils::AsyncMsgQueue::new();

        let msg_fut = async {
            let l = msg_buf.len();
            let (msg_encoded, msg_decoded) = msg_buf.split_at_mut(l / 2 + 1);

            let mut ts = crate::TrackerState::new();
            if let Some(len) = crate::update_tracker_state(&mut ts, true, false, msg_decoded) {
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

            let interval = crate::milliseconds_to_ticks(5000) as u64;
            let mut next_full_update = primeapi::dol_sdk::os::OSGetTime() + interval;
            loop {
                crate::delay(crate::milliseconds_to_ticks(2500)).await;

                let curr_time = primeapi::dol_sdk::os::OSGetTime();
                let full_update = curr_time > next_full_update;
                if full_update {
                    next_full_update = curr_time + interval;
                }

                let len = if let Some(len) = crate::update_tracker_state(&mut ts, false, full_update, msg_decoded) {
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
            // Every ~10 seconds send a ping
            loop {
                crate::delay(crate::milliseconds_to_ticks(10000)).await;
                let res = ws.borrow_mut()
                    .write(WebSocketSendMessageType::Ping, true, &[], ping_buf);
                match res {
                    Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                    Err(e) => { primeapi::dbg!(e); },
                }
                let res = ws.borrow_mut()
                    .write(WebSocketSendMessageType::Text, true, b"{\"ping\":null}", ping_buf);
                match res {
                    Ok(i) => write_queue.sync_push(&ping_buf[..i]).await,
                    Err(e) => { primeapi::dbg!(e); },
                }
            }
        };

        let reader_fut = async {
            let l = recv_buf.len();
            let (recv_encoded, recv_decoded) = recv_buf.split_at_mut(l / 2 + 1);
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
                        let buf = <[MaybeUninit<u8>]>::from_inited_slice_mut(recv_encoded);
                        let fut = reader.async_read(buf);
                        pin_mut!(fut);
                        recv_encoded_len += fut.rebound_pinned().await.map_err(|_| ())?;
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
                        .write(WebSocketSendMessageType::Pong, true, &[], recv_decoded)
                        .map_err(|e| { primeapi::dbg!(e); })?;
                        // .map_err(|_| ())?;
                    write_queue.sync_push(&recv_decoded[..l]).await;
                } else if res.message_type == WebSocketReceiveMessageType::CloseMustReply {
                    let l = ws.borrow_mut()
                        .write(WebSocketSendMessageType::CloseReply, true, &[], recv_decoded)
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
                async_utils::async_write_all(&mut writer, &buf_ref).await?;
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
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HttpRequestError<RE, WE>
{
    ReaderIO(RE),
    WriterIO(WE),
    Internal,
}

pub async fn handle_http_request<R, W>(
    buf: &mut [MaybeUninit<u8>],
    mut sock_reader: R,
    mut sock_writer: W,
) -> Result<bool, HttpRequestError<R::Error, W::Error>>
    where R: AsyncRead,
          W: AsyncWrite,
        W::Error: core::fmt::Debug,
{
    let fs = DvdFileSystem;
    let mut ws = WebSocketHandler;
    let res: Result<_, HttpError<R::Error, W::Error>> = try {
        let mut handler = SupportedHttpHeaderFields::default();
        let already_read_len = {
            let mut lr = LineReader::with_buf(&mut *buf, &mut sock_reader);
            parse_http_request_headers(&mut lr, &mut handler).await?;
            lr.peek_buf().len()
        };

        // TODO: It might be better to just error out if we receive any payload
        // We never actually care about the payload, so skip it if it exists
        // We'd have to immediatley close the connection, which is not ideal, I suppose
        if let Some(content_length) = &handler.content_length {
            let mut content_length = content_length - already_read_len;

            while content_length > 0 {
                let len = core::cmp::min(buf.len(), content_length);
                let fut = sock_reader.async_read(&mut buf[..len]);
                pin_mut!(fut);
                content_length -= fut.rebound_pinned().await.map_err(HttpError::ReaderIO)?;
            }
        }

        // Check if this is a websocket request
        if handler.upgrade_websocket {
            // Verify we received all of the required headers
            let key = match handler.websocket_key {
                Some(key) if handler.connection_upgrade && handler.websocket_version_13 => key,
                _ => Err(HttpError::semantic(HttpStatus::BadRequest400, ""))?,
            };

            let maybe_fut = ws.websocket_connection(
                &handler.uri,
                key,
                sock_reader,
                { sock_writer },
                &mut *buf
            );
            sock_writer = match maybe_fut {
                Ok(fut) => {
                    fut.await;
                    return Ok(false);
                },
                Err(writer) => writer,
            };
            Err(HttpError::semantic(HttpStatus::NotFound404, ""))?
        }

        // Lookup the file
        let (metadata, mut file_reader) = fs.lookup_file(&handler.uri)
            .ok_or_else(|| HttpError::semantic(HttpStatus::NotFound404, ""))?;

        // Check if we can skip sending the payload
        let last_modified = metadata.last_modified.and_then(HttpDate::from_str);
        let use_cached = match ((&handler.if_none_match, &metadata.etag),
                                (handler.if_modified_since, last_modified)) {
            ((Some(e0), Some(e1)), _) if e0 == e1 => true,
            (_, (Some(d0), Some(d1))) if d0 == d1 => true,
            _ => false,
        };

        if use_cached {
            let mut buf_writer = BufferedAsyncWriter::with_buf(&mut *buf, &mut sock_writer);

            buf_writer.write("HTTP/1.1 ".as_bytes()).await.map_err(HttpError::WriterIO)?;
            buf_writer.write(HttpStatus::NotModified304.header_message().as_bytes()).await
                .map_err(HttpError::WriterIO)?;
            buf_writer.write(b"Connection: close\r\n").await.map_err(HttpError::WriterIO)?;
            buf_writer.write(b"\r\n").await.map_err(HttpError::WriterIO)?;
            buf_writer.flush().await.map_err(HttpError::WriterIO)?;
        } else {
            let mut buf_writer = BufferedAsyncWriter::with_buf(&mut *buf, &mut sock_writer);

            buf_writer.write("HTTP/1.1 ".as_bytes()).await.map_err(HttpError::WriterIO)?;
            buf_writer.write(HttpStatus::OK200.header_message().as_bytes()).await
                .map_err(HttpError::WriterIO)?;

            let mut line_buf = ArrayString::<[u8; 100]>::new();

            writeln!(line_buf, "Content-Length: {}\r", metadata.size)
                .map_err(|_| HttpError::Unrecoverable)?;
            buf_writer.write(line_buf.as_bytes()).await.map_err(HttpError::WriterIO)?;
            line_buf.clear();

            if let Some(last_modified) = metadata.last_modified {
                writeln!(line_buf, "Last-Modified: {}\r", last_modified)
                    .map_err(|_| HttpError::Unrecoverable)?;
                buf_writer.write(line_buf.as_bytes()).await.map_err(HttpError::WriterIO)?;
                line_buf.clear();
            }

            if let Some(etag) = metadata.etag {
                if let Ok(s) = core::str::from_utf8(&etag) {
                    writeln!(line_buf, "ETag: \"{}\"\r", s)
                        .map_err(|_| HttpError::Unrecoverable)?;
                    buf_writer.write(line_buf.as_bytes()).await.map_err(HttpError::WriterIO)?;
                    line_buf.clear();
                }
            }

            buf_writer.write(b"Connection: close\r\n").await.map_err(HttpError::WriterIO)?;

            buf_writer.write(b"\r\n").await.map_err(HttpError::WriterIO)?;
            buf_writer.flush().await.map_err(HttpError::WriterIO)?;
        }

        if handler.method == Some(HttpMethod::Get) {
            // Actually send the payload
            let mut bytes_to_read = metadata.size;
            while bytes_to_read > 0 {
                let i = {
                    let fut = file_reader.async_read(&mut buf[..]);
                    pin_mut!(fut);
                    fut.await
                        .map_err(|_| HttpError::Unrecoverable)?
                };

                bytes_to_read -= i as u32;
                async_write_all(&mut sock_writer, unsafe { buf[..i].assume_init() }).await
                    .map_err(HttpError::WriterIO)?;

            }
        }

        false
    };

    match res {
        Ok(b) => Ok(b),
        Err(HttpError::Semantic(HttpSemanticError(status, _msg))) => {
            let res: Result<_, W::Error> = async {
                // TODO Actually write out _msg
                let mut buf_writer = BufferedAsyncWriter::with_buf(&mut *buf, &mut sock_writer);
                buf_writer.write("HTTP/1.1 ".as_bytes()).await?;
                buf_writer.write(status.header_message().as_bytes()).await?;
                buf_writer.write(b"Connection: close\r\n").await?;
                buf_writer.write(b"\r\n").await?;
                buf_writer.flush().await?;
                Ok(false)
            }.await;
            res.map_err(|e| HttpRequestError::WriterIO(e))
        },
        Err(HttpError::ReaderIO(e)) => { Err(HttpRequestError::ReaderIO(e)) },
        Err(HttpError::WriterIO(e)) => { Err(HttpRequestError::WriterIO(e)) },
        Err(HttpError::Unrecoverable) => Err(HttpRequestError::Internal),
    }
}
