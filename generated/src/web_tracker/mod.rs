use flate2::{read, write};

use std::io::{self, Read, Write};
use reader_writer::WithRead;

#[derive(Debug, Clone)]
pub struct File
{
    pub name: &'static str,
    pub decompressed_size: usize,
    pub compressed_bytes: &'static [u8],
}

impl File
{
    pub fn decompress(&self) -> Vec<u8>
    {
        let mut decoder = write::DeflateDecoder::new(vec![]);
        decoder.write_all(self.compressed_bytes).unwrap();
        decoder.finish().unwrap()
    }
}

impl WithRead for File
{
    fn len(&self) -> usize
    {
        self.decompressed_size
    }

    fn boxed<'r>(&self) -> Box<dyn WithRead + 'r>
        where Self: 'r
    {
        Box::new(self.clone())
    }

    fn with_read(&self, f: &mut dyn FnMut(&mut dyn Read) -> io::Result<u64>) -> io::Result<u64>
    {
        let mut decoder = read::DeflateDecoder::new(io::Cursor::new(self.compressed_bytes));
        f(&mut decoder)
    }

}

include!(concat!(env!("OUT_DIR"), "/web_tracker_generated.rs"));
