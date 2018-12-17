use reader_writer::byteorder::{LittleEndian, WriteBytesExt};

use std::{
    cmp::min,
    io::{self, Seek, Write},
    iter,
};

use crate::gcz_writer::ZEROES;

// Implementation examples
// https://github.com/FIX94/Nintendont/blob/3e81dadcfc4b19129f08a947905331f1d45a1b0b/kernel/ISO.c
// https://github.com/dolphin-emu/dolphin/blob/8f460a1cda1a4d4208c4da9e01bf775f5f704498/Source/Core/DiscIO/CISOBlob.h
// https://github.com/dolphin-emu/dolphin/blob/8f460a1cda1a4d4208c4da9e01bf775f5f704498/Source/Core/DiscIO/CISOBlob.cpp

// const CISO_MAGIC: u32 = 0x4349534F; 'CISO'
const HEADER_SIZE: usize = 0x8000;

macro_rules! block_size {
    () => { 2 * 1024 * 1024 };
}
const BLOCK_SIZE: u32 = block_size!();

pub struct CisoWriter<W: Write + Seek>
{
    file: W,
    blocks_map: Vec<u8>,
    skipped_blocks: u32,
}

impl<W: Write + Seek> CisoWriter<W>
{
    pub fn new(mut file: W) -> io::Result<CisoWriter<W>>
    {
        file.seek(io::SeekFrom::Start(0))?;
        file.write_all(&[0u8; HEADER_SIZE])?;
        Ok(CisoWriter {
            file,
            blocks_map: Vec::with_capacity(HEADER_SIZE - 8),
            skipped_blocks: 0,
        })
    }

    // pub fn new(mut file: W) -> io::Result<CisoWriter<W>>
    fn write_zeroes(&mut self, mut bytes: u64) -> io::Result<()>
    {
        while bytes > 0 {
            let l = min(ZEROES.len() as u64, bytes);
            self.file.write_all(&ZEROES[..l as usize])?;
            bytes -= l;
        }
        Ok(())
    }
}

impl<W: Write + Seek> Write for CisoWriter<W>
{
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize>
    {
        self.file.write(bytes)
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>
    {
        self.file.write_all(bytes)
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.file.flush()
    }
}

impl<W: Write + Seek + 'static> structs::WriteExt for CisoWriter<W>
{
    fn skip_bytes(&mut self, bytes: u64) -> io::Result<()>
    {
        let pos = self.file.seek(io::SeekFrom::Current(0))?;
        let pos_rounded_up = (pos + block_size!() - 1) & !(block_size!() - 1);

        // Finish out the current block with zeroes
        let extra = min(pos_rounded_up - pos, bytes);
        self.write_zeroes(extra)?;
        let bytes = bytes - extra;

        // Update the block map to reflect all of the used blocks so far
        let current_block = pos_rounded_up / block_size!() + self.skipped_blocks as u64;
        let l = current_block as usize - self.blocks_map.len();
        self.blocks_map.extend(iter::repeat(1).take(l));

        // Add skipped blocks
        let to_skip = bytes / block_size!();
        self.blocks_map.extend(iter::repeat(0).take(to_skip as usize));
        self.skipped_blocks += to_skip as u32;

        // Fill in the start of the next block with zeroes
        self.write_zeroes(bytes % block_size!())?;

        Ok(())

    }
}

impl<W: Write + Seek> Drop for CisoWriter<W>
{
    fn drop(&mut self)
    {
        let res = || -> io::Result<()> {
            let pos = self.file.seek(io::SeekFrom::Current(0))?;
            let pos_rounded_up = (pos + block_size!() - 1) & !(block_size!() - 1);
            let current_block = pos_rounded_up / block_size!() + self.skipped_blocks as u64;
            let l = current_block as usize - self.blocks_map.len();
            self.blocks_map.extend(iter::repeat(1).take(l));

            // Write header (We can use Writable because of big-endianness)
            self.file.seek(io::SeekFrom::Start(0))?;
            self.file.write_all(b"CISO")?;
            self.file.write_u32::<LittleEndian>(BLOCK_SIZE)?;
            self.file.write_all(&self.blocks_map[..])?;
            Ok(())
        }();
        // We really don't want to panic from a destructor, so just write a warning instead
        if let Err(e) = res {
            eprintln!("Error closing GczWriter: {}", e);
        };
    }
}
