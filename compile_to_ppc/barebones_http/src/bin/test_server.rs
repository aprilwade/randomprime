use std::future::Future;
use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::net::TcpListener;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::path::Path;
use std::fs::File;

use async_utils::{
    poll_until_complete, AsyncRead, AsyncWrite, Lifetime1Rebinder, Rebind1Lifetime,
    MaybeUninitSliceExt
};
use barebones_http::{handle_http_request, FileMetadata, FileSystemSource};

#[derive(Copy, Clone, Debug)]
struct IoWriteAdaptor<W>(W);
#[derive(Debug)]
struct IoWriteAdaptorFuture<'a, W>(*mut W, &'a [u8]);

impl<W> AsyncWrite for IoWriteAdaptor<W>
    where W: Write
{
    type Error = io::Error;
    type Future = IoWriteAdaptorFuture<'static, W>;

    fn async_write<'a>(&'a mut self, buf: &'a [u8]) -> Lifetime1Rebinder<'a, Self::Future>
    {
        Lifetime1Rebinder::new(IoWriteAdaptorFuture(&mut self.0, buf))
    }
}

impl<'a, W> Future for IoWriteAdaptorFuture<'a, W>
    where W: Write
{
    type Output = io::Result<usize>;
    fn poll(mut self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output>
    {
        let buf = self.1;
        Poll::Ready(unsafe { &mut *self.0 }.write(buf))
    }
}

impl<'a, W> Rebind1Lifetime<'a> for IoWriteAdaptorFuture<'static, W>
    where W: Write
{
    type Rebound = IoWriteAdaptorFuture<'a, W>;
}

#[derive(Copy, Clone, Debug)]
struct IoReadAdaptor<R>(R);
// #[derive(Debug)]
struct IoReadAdaptorFuture<'a, R>(*mut R, &'a mut [MaybeUninit<u8>]);

impl<R> AsyncRead for IoReadAdaptor<R>
    where R: Read
{
    type Error = io::Error;
    type Future = IoReadAdaptorFuture<'static, R>;

    fn async_read<'a>(&'a mut self, buf: &'a mut [MaybeUninit<u8>])
        -> Lifetime1Rebinder<'a, Self::Future>
    {
        // XXX Just to be safe, initialize all of the bytes in the slice
        unsafe { std::ptr::write_bytes(buf.as_mut_ptr() as *mut u8, 0, buf.len()) }
        Lifetime1Rebinder::new(IoReadAdaptorFuture(&mut self.0, buf))
    }
}

impl<'a, R> Future for IoReadAdaptorFuture<'a, R>
    where R: Read
{
    type Output = io::Result<usize>;
    fn poll(mut self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output>
    {
        let IoReadAdaptorFuture(reader, buf) = &mut *self;
        Poll::Ready(unsafe { &mut **reader }.read(unsafe { buf.assume_init_mut() }))
    }
}

impl<'a, R> Rebind1Lifetime<'a> for IoReadAdaptorFuture<'static, R>
    where R: Read
{
    type Rebound = IoReadAdaptorFuture<'a, R>;
}

struct DirFileSystem<'a>(&'a Path);

impl<'a> FileSystemSource for DirFileSystem<'a>
{
    type Reader = IoReadAdaptor<File>;
    fn lookup_file(&self, uri: &[u8]) -> Option<(FileMetadata, Self::Reader)>
    {
        let uri = std::str::from_utf8(uri).ok()?;
        let path = self.0.join(uri.trim_matches('/'));
        println!("{:?}", path);
        let fs_metadata = path.metadata().ok()?;

        if fs_metadata.is_dir() {
            return None
        }

        let etag_raw = fs_metadata.modified().unwrap()
            .duration_since(std::time::UNIX_EPOCH).unwrap()
            .as_secs();
        let mut etag = [b'0'; 16];

        write!(std::io::Cursor::new(&mut etag[..]), "{:016x}", etag_raw).unwrap();

        let metadata = FileMetadata {
            size: fs_metadata.len() as u32,
            etag: Some(etag),
            last_modified: None,
        };
        let file = File::open(path).ok()?;
        Some((metadata, IoReadAdaptor(file)))
    }
}

fn main()
{
    let fs = DirFileSystem(Path::new("/Users/aprilwade/workspace/prime_randomizer/randomprime/web/"));
    let listener = TcpListener::bind("127.0.0.1:59595").unwrap();
    for tcp_stream in listener.incoming() {
        let tcp_stream = tcp_stream.unwrap();
        println!("{:?}", poll_until_complete(handle_http_request(
            IoReadAdaptor(&tcp_stream),
            IoWriteAdaptor(&tcp_stream),
            &fs
        )));
    }
}
