use reader_writer::byteorder::{LittleEndian, WriteBytesExt};
use structs;

use flate2::{self, Compress, Compression, FlushCompress};
use adler32::adler32;

use std::{
    cmp::min,
    io::{self, Seek, Write},
};

// constants are fixed to one integer type...
macro_rules! block_size {
    () => { 16 * 1024 }
}

// const BLOCK_SIZE: u64 = block_size!();
const GCZ_MAGIC: u32 = 0xB10BC001;

pub const ZEROES: &[u8; block_size!()] = &[0u8; block_size!()];

pub struct GczWriter<W: Write + Seek>
{
    expected_uncompressed_size: u64,
    total_bytes_written: u64,
    block_offsets: Vec<u64>,
    hashes: Vec<u32>,

    input_buf_used: u32,
    input_buf: [u8; block_size!()],
    output_buf: [u8; block_size!()],

    zero_block_data: Option<(Vec<u8>, u32)>,// (bytes, hash)

    compressor: Compress,
    file: W,
}

impl<W: Write + Seek> GczWriter<W>
{
    pub fn new(mut file: W, uncompressed_size: u64) -> io::Result<Box<GczWriter<W>>>
    {
        file.seek(io::SeekFrom::Start(0))?;

        let num_blocks = ((uncompressed_size + block_size!() - 1) / block_size!()) as usize;
        let mut header_bytes = 32 + 12 * num_blocks;
        while header_bytes > 0 {
            let l = min(block_size!(), header_bytes);
            file.write_all(&ZEROES[..l])?;
            header_bytes -= l;
        }

        Ok(Box::new(GczWriter {
            expected_uncompressed_size: uncompressed_size,

            total_bytes_written: 0,
            block_offsets: Vec::with_capacity(num_blocks),
            hashes: Vec::with_capacity(num_blocks),

            input_buf_used: 0,
            input_buf: [0u8; block_size!()],
            output_buf: [0u8; block_size!()],

            zero_block_data: None,

            compressor: Compress::new(Compression::best(), true),
            file,
        }))
    }
}


impl<W: Write + Seek> Write for GczWriter<W>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        self.write_all(buf).map(|()| buf.len())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()>
    {
        while buf.len() as u64 + self.input_buf_used as u64 >= block_size!() {
            let (left_buf, right_buf) = buf.split_at(block_size!() - self.input_buf_used as usize);
            self.input_buf[self.input_buf_used as usize..block_size!()].copy_from_slice(left_buf);

            self.compressor.reset();
            let res = self.compressor.compress(&self.input_buf, &mut self.output_buf,
                                               FlushCompress::Finish).unwrap();
            let finished = res == flate2::Status::StreamEnd;
            let compressed_len = self.compressor.total_out();
            let output_buf = &self.output_buf[..compressed_len as usize];

            if !finished || compressed_len > block_size!() - 10 {
                self.block_offsets.push(self.total_bytes_written | 0x8000000000000000);
                self.file.write_all(&self.input_buf)?;
                self.total_bytes_written += block_size!();
                self.hashes.push(adler32(&self.input_buf[..])?);
            } else {
                self.block_offsets.push(self.total_bytes_written);
                self.file.write_all(&output_buf)?;
                self.total_bytes_written += compressed_len as u64;
                self.hashes.push(adler32(&output_buf[..])?);
            }

            self.input_buf_used = 0;
            buf = right_buf;
        }


        let rng = self.input_buf_used as usize..buf.len() + self.input_buf_used as usize;
        self.input_buf[rng].copy_from_slice(buf);
        self.input_buf_used += buf.len() as u32;

        Ok(())
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.file.flush()
    }
}

impl<W: Write + Seek> structs::WriteExt for GczWriter<W>
{
    fn skip_bytes(&mut self, mut bytes: u64) -> io::Result<()>
    {
        if bytes < block_size!() {
            // We have less than a full block of zeros so we can't do better than just naively
            // writing zeroes
            return self.write_all(&ZEROES[..bytes as usize]);
        }

        if self.input_buf_used != 0 {
            // Finish the current block with zeroes
            let l = block_size!() - self.input_buf_used as usize;
            self.write_all(&ZEROES[..l])?;
            bytes -= l as u64;
        }

        while bytes > block_size!() {
            // Instead of compresssing all of these zeroes repeatedly, just reuse a precalculated
            // zero block.
            if self.zero_block_data.is_none() {
                self.compressor.reset();
                let res = self.compressor.compress(&ZEROES[..], &mut self.output_buf,
                                                FlushCompress::Finish).unwrap();
                assert!(res == flate2::Status::StreamEnd);
                let compressed_len = self.compressor.total_out() as usize;
                let compressed_bytes = self.output_buf[..compressed_len].to_owned();
                let hash = adler32(&compressed_bytes[..])?;
                self.zero_block_data = Some((compressed_bytes, hash));
            }
            let (compressed_bytes, hash) = self.zero_block_data.as_ref().unwrap();
            self.block_offsets.push(self.total_bytes_written);
            self.total_bytes_written += compressed_bytes.len() as u64;
            self.hashes.push(*hash);
            self.file.write_all(&compressed_bytes[..])?;

            bytes -= block_size!();
        }

        // Write leftover zeroes
        self.write_all(&ZEROES[..bytes as usize])
    }
}

impl<W: Write + Seek> Drop for GczWriter<W>
{
    fn drop(&mut self)
    {
        let res = || -> io::Result<()> {
            // Write whatever is left over in our buffer to a block (empty space paddeded with zeroes)
            if self.input_buf_used != 0 {
                let bytes_to_zero = block_size!() - self.input_buf_used as usize;
                self.write_all(&ZEROES[..bytes_to_zero])?;
            }

            assert!(self.input_buf_used == 0);

            // Seek the file back to the start and write the header
            self.file.seek(io::SeekFrom::Start(0))?;
            self.file.write_u32::<LittleEndian>(GCZ_MAGIC)?;
            self.file.write_u32::<LittleEndian>(0)?;
            self.file.write_u64::<LittleEndian>(self.total_bytes_written)?;
            self.file.write_u64::<LittleEndian>(self.expected_uncompressed_size)?;
            self.file.write_u32::<LittleEndian>(block_size!())?;
            self.file.write_u32::<LittleEndian>(self.block_offsets.len() as u32)?;
            for offset in &self.block_offsets {
                self.file.write_u64::<LittleEndian>(*offset)?;
            }
            for hash in &self.hashes {
                self.file.write_u32::<LittleEndian>(*hash)?;
            }
            Ok(())
        }();
        // We really don't want to panic from a destructor, so just write a warning instead
        if let Err(e) = res {
            eprintln!("Error closing GczWriter: {}", e);
        };
    }
}
