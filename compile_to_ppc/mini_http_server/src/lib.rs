#![cfg_attr(not(test), no_std)]
#![feature(try_blocks, new_uninit)]

extern crate alloc;

use futures::future::Either;

use alloc::borrow::{Cow, ToOwned};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use core::default::Default;
use core::future::Future;
use core::mem::MaybeUninit;
use ufmt::uwriteln;

use async_utils:: io::{
    AsyncIoError, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, LineReader, LineReaderError,
};


struct StringuWriteAdaptor(String);
impl ufmt::uWrite for StringuWriteAdaptor
{
    type Error = core::convert::Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.0.push_str(s);
        Ok(())
    }
}

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
    where R: AsyncRead + Unpin,
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

pub trait FileSystem
{
    type File: AsyncBufRead;
    fn open_file(&self, uri: &[u8]) -> Option<(Self::File, FileMetadata)>;
}

pub struct FileMetadata
{
    pub size: u32,
    pub last_modified: Option<&'static str>,
    pub etag: Option<[u8; 16]>,
}

#[derive(Clone, Copy, Debug)]
pub enum HttpRequestError<RE, WE>
{
    ReaderIO(RE),
    WriterIO(WE),
    Internal,
}

#[derive(Clone, Copy, Debug)]
pub struct HttpRequestHandler<R, W, FS, WS>
{
    reader: R,
    writer: W,
    fs: FS,
    ws: WS,
}

impl HttpRequestHandler<(), (), (), ()>
{
    pub fn new() -> Self
    {
        HttpRequestHandler {
            reader: (),
            writer: (),
            fs: (),
            ws: (),
        }
    }
}

impl<R, W, FS, WS> HttpRequestHandler<R, W, FS, WS>
{
    pub fn reader<R2: AsyncRead>(self, r: R2) -> HttpRequestHandler<R2, W, FS, WS>
    {
        HttpRequestHandler {
            reader: r,
            writer: self.writer,
            fs: self.fs,
            ws: self.ws,
        }
    }

    pub fn writer<W2: AsyncWrite>(self, w: W2) -> HttpRequestHandler<R, W2, FS, WS>
    {
        HttpRequestHandler {
            reader: self.reader,
            writer: w,
            fs: self.fs,
            ws: self.ws,
        }
    }

    pub fn fs<FS2: FileSystem>(self, fs: FS2) -> HttpRequestHandler<R, W, FS2, WS>
    {
        HttpRequestHandler {
            reader: self.reader,
            writer: self.writer,
            fs: fs,
            ws: self.ws,
        }
    }

    pub fn ws<WS2, WSE, WSF>(self, ws: WS2) -> HttpRequestHandler<R, W, FS, WS2>
        where WS2: FnOnce(&[u8], [u8; 24], R, W) -> Result<WSF, W>,
              WSF: Future<Output = Result<(), WSE>>,
    {
        HttpRequestHandler {
            reader: self.reader,
            writer: self.writer,
            fs: self.fs,
            ws: ws,
        }
    }
}

use async_utils::io::AsyncBufRead;

impl<R, W, FS, WS, WSF, WSE> HttpRequestHandler<R, W, FS, WS>
    where R: AsyncRead + Unpin,
          W: AsyncWrite + Unpin,
          R::Error: core::fmt::Debug,
          W::Error: core::fmt::Debug + AsyncIoError,
          FS: FileSystem,
          WS: FnOnce(&[u8], [u8; 24], R, W) -> Result<WSF, W>,
          WSF: Future<Output = Result<(), WSE>>,
{
    pub async fn handle_http_request(self) -> Result<bool, HttpRequestError<R::Error, W::Error>>
    {
        let Self { mut reader, mut writer, fs, ws } = self;

        let res: Result<_, HttpError<R::Error, W::Error>> = try {
            let mut handler = SupportedHttpHeaderFields::default();
            let already_read_len = {
                let mut lr = (&mut reader).line_reader(Box::new_uninit_slice(512));
                parse_http_request_headers(&mut lr, &mut handler).await?;
                lr.peek_buf().len()
            };

            // TODO: It might be better to just error out if we receive any payload
            // We never actually care about the payload, so skip it if it exists
            // We'd have to immediatley close the connection, which is not ideal, I suppose
            if let Some(content_length) = &handler.content_length {
                let mut content_length = content_length - already_read_len;

                let mut buf = Box::new_uninit_slice(512);
                while content_length > 0 {
                    let len = core::cmp::min(buf.len(), content_length);
                    let fut = (&mut reader).read(&mut buf[..len]);
                    content_length -= fut.await.map_err(HttpError::ReaderIO)?;
                }
            }

            // Check if this is a websocket request
            if handler.upgrade_websocket {
                // Verify we received all of the required headers
                let key = match handler.websocket_key {
                    Some(key) if handler.connection_upgrade && handler.websocket_version_13 => key,
                    _ => Err(HttpError::semantic(HttpStatus::BadRequest400, ""))?,
                };

                let maybe_fut = (ws)(
                    &handler.uri,
                    key,
                    reader,
                    writer,
                );
                writer = match maybe_fut {
                    Ok(fut) => {
                        // TODO: We should have a way to propgate this error
                        let _ = fut.await;
                        return Ok(false);
                    },
                    Err(writer) => writer,
                };
                Err(HttpError::semantic(HttpStatus::NotFound404, ""))?
            }

            // Lookup the file
            let (file, metadata) = fs.open_file(&handler.uri)
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
                let mut buf_writer = (&mut writer).buf_writer(Box::new_uninit_slice(512));

                buf_writer.write_all("HTTP/1.1 ".as_bytes()).await.map_err(HttpError::WriterIO)?;
                buf_writer.write_all(HttpStatus::NotModified304.header_message().as_bytes()).await
                    .map_err(HttpError::WriterIO)?;
                buf_writer.write_all(b"Connection: close\r\n").await.map_err(HttpError::WriterIO)?;
                buf_writer.write_all(b"\r\n").await.map_err(HttpError::WriterIO)?;
                buf_writer.flush().await.map_err(HttpError::WriterIO)?;
            } else {
                let mut buf_writer = (&mut writer).buf_writer(Box::new_uninit_slice(512));

                buf_writer.write_all("HTTP/1.1 ".as_bytes()).await.map_err(HttpError::WriterIO)?;
                buf_writer.write_all(HttpStatus::OK200.header_message().as_bytes()).await
                    .map_err(HttpError::WriterIO)?;

                let mut line_buf = StringuWriteAdaptor(String::with_capacity(50));

                // TODO: Can I do this without the seperate buffer? Like, flush the previous
                //       message first? Or use the initial buf_writer buffer instead?
                uwriteln!(line_buf, "Content-Length: {}\r", metadata.size)
                    .map_err(|_| HttpError::Unrecoverable)?;
                buf_writer.write_all(line_buf.0.as_bytes()).await.map_err(HttpError::WriterIO)?;
                line_buf.0.clear();

                if let Some(last_modified) = metadata.last_modified {
                    uwriteln!(line_buf, "Last-Modified: {}\r", last_modified)
                        .map_err(|_| HttpError::Unrecoverable)?;
                    buf_writer.write_all(line_buf.0.as_bytes()).await.map_err(HttpError::WriterIO)?;
                    line_buf.0.clear();
                }

                if let Some(etag) = metadata.etag {
                    if let Ok(s) = core::str::from_utf8(&etag) {
                        uwriteln!(line_buf, "ETag: \"{}\"\r", s)
                            .map_err(|_| HttpError::Unrecoverable)?;
                        buf_writer.write_all(line_buf.0.as_bytes()).await
                            .map_err(HttpError::WriterIO)?;
                        line_buf.0.clear();
                    }
                }

                let _ = line_buf;

                buf_writer.write_all(b"Connection: close\r\n").await.map_err(HttpError::WriterIO)?;

                buf_writer.write_all(b"\r\n").await.map_err(HttpError::WriterIO)?;
                buf_writer.flush().await.map_err(HttpError::WriterIO)?;
            }

            if handler.method == Some(HttpMethod::Get) {
                // Actually send the payload
                async_utils::io::copy_buf(file, &mut writer).await
                    .map_err(|e| match e {
                        Either::Left(_) => HttpError::Unrecoverable,
                        Either::Right(e) => HttpError::WriterIO(e),
                    })?;
            }

            false
        };

        match res {
            Ok(b) => Ok(b),
            Err(HttpError::Semantic(HttpSemanticError(status, _msg))) => {
                let res: Result<_, W::Error> = async {
                    // TODO Actually write out _msg
                    let mut buf_writer = writer.buf_writer(Box::new_uninit_slice(512));
                    buf_writer.write_all("HTTP/1.1 ".as_bytes()).await?;
                    buf_writer.write_all(status.header_message().as_bytes()).await?;
                    buf_writer.write_all(b"Connection: close\r\n").await?;
                    buf_writer.write_all(b"\r\n").await?;
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
}

#[cfg(test)]
mod test
{
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    use async_utils::{poll_until_complete, io::Cursor};

    use futures::future::Ready;

    #[derive(Clone)]
    struct TestingFileSystem
    {
        files: Vec<(&'static str, &'static [u8])>,
        etag: Option<[u8; 16]>,
        last_modified: Option<&'static str>,
    }

    impl FileSystem for TestingFileSystem
    {
        type File = Cursor<&'static [u8]>;
        fn open_file(&self, uri: &[u8]) -> Option<(Self::File, FileMetadata)>
        {
            self.files.iter()
                .find(|(name, _)| name.as_bytes() == uri)
                .map(|(_, data)| (
                        Cursor::new(*data),
                        FileMetadata {
                            size: data.len() as u32,
                            etag: self.etag,
                            last_modified: self.last_modified,
                        },
                    ))
        }
    }

    #[test]
    fn test_handle_http_request()
    {
        let fs = TestingFileSystem {
            files: vec![
                ("/testing", b"testing"),
                ("/binary_data", b"\x01\x02\x03\x04"),
            ],
            etag: None,
            last_modified: None,
        };

        let request = "\
            GET /testing HTTP/1.1\r\n\
            Host: example.com\r\n\
            \r\n";
        let mut writer = vec![];
        let handler = HttpRequestHandler::new()
            .reader(Cursor::new(request.as_bytes()))
            .writer(&mut writer)
            .fs(fs.clone())
            .ws(|_, _, _, w| -> Result<Ready<Result<(), ()>>, _> { Err(w) });
        poll_until_complete(handler.handle_http_request()).unwrap();
        assert_eq!(&writer[..], "\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 7\r\n\
            Connection: close\r\n\
            \r\n\
            testing".as_bytes()
        );

        let request = "\
            HEAD /testing HTTP/1.1\r\n\
            Host: example.com\r\n\
            \r\n";
        let mut writer = vec![];
        let handler = HttpRequestHandler::new()
            .reader(Cursor::new(request.as_bytes()))
            .writer(&mut writer)
            .fs(fs.clone())
            .ws(|_, _, _, w| -> Result<Ready<Result<(), ()>>, _> { Err(w) });
        poll_until_complete(handler.handle_http_request()).unwrap();
        assert_eq!(&writer[..], "\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 7\r\n\
            Connection: close\r\n\
            \r\n".as_bytes()
        );

        let request = "\
            GET /unknown HTTP/1.1\r\n\
            Host: example.com\r\n\
            \r\n";
        let mut writer = vec![];
        let handler = HttpRequestHandler::new()
            .reader(Cursor::new(request.as_bytes()))
            .writer(&mut writer)
            .fs(fs.clone())
            .ws(|_, _, _, w| -> Result<Ready<Result<(), ()>>, _> { Err(w) });
        poll_until_complete(handler.handle_http_request()).unwrap();
        assert_eq!(&writer[..], "\
            HTTP/1.1 404 Not Found\r\n\
            Connection: close\r\n\
            \r\n".as_bytes()
        );

        // TODO: Test cache-related behavior
    }

    #[test]
    fn test_http_date()
    {
        assert_eq!(
            HttpDate::from_str("Tue, 17 Sep 2019 21:55:30 GMT").unwrap(),
            HttpDate::from_parts(2019, 9, 17, 21, 55, 30)
        );
    }

}
