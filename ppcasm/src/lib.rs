pub use ppcasm_macro::ppcasm;

use std::io;

use byteorder::{BigEndian, WriteBytesExt};
pub use generic_array;

pub fn upper_bits(n: u32) -> i32
{
    if n & 0x8000 == 0x8000 {
        ((n >> 16) as i32) + 1
    } else {
        (n >> 16) as i32
    }
}

pub fn lower_bits(n: u32) -> i32
{
    (n & 0xFFFF) as i32
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsmBlock<A, L>
{
    addr: u32,
    instrs: A,
    labels: L,
}

impl<A, L> AsmBlock<A, L>
    where A: AsRef<[u32]>
{
    pub fn new(addr: u32, instrs: A, labels: L) -> AsmBlock<A, L>
    {
        AsmBlock { addr, instrs, labels }
    }

    pub fn write_encoded<W: io::Write>(&self, w: &mut W) -> io::Result<()>
    {
        for instr in self.instrs.as_ref().iter() {
            w.write_u32::<BigEndian>(*instr)?
        }
        Ok(())
    }

    pub fn encoded_bytes(&self) -> Vec<u8>
    {
        let mut v = Vec::with_capacity(self.instrs.as_ref().len() * 4);
        self.write_encoded(&mut v).unwrap();
        v
    }

    pub fn addr(&self) -> u32
    {
        self.addr
    }

    pub fn labels(&self) -> &L
    {
        &self.labels
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AsmInstrPart(u8, u32);
impl AsmInstrPart
{
    pub fn new<T: Into<i64>>(bit_len: u8, data: T) -> AsmInstrPart
    {
        if bit_len > 32 {
            panic!("AsmInstrPart bit_len may not exceed 32 (max 32 bits per instruction)")
        }

        let data = data.into();
        let data = if data >= 0 {
            if (1 << bit_len) - 1 < data {
                panic!("AsmInstrPart value {:x} is longer than {} bits", data, bit_len)
            }
            data as u32
        } else {
            // NOTE: (bit_len - 1) because the top bit MUST be negative in 2s complement
            if -1 << (bit_len - 1) > data {
                panic!("AsmInstrPart value {:x} is longer than {} bits", data, bit_len)
            }
            if bit_len == 32 {
                data as u32
            } else {
                // Zero out the upper bits
                data as u32 & ((1 << bit_len) - 1)
            }
        };
        AsmInstrPart(bit_len, data)
    }

    pub fn bit_len(&self) -> u8
    {
        self.0
    }

    fn shiftable_bit_len(&self) -> u8
    {
        if self.0 == 32 { 0 } else { self.0 }
    }


    pub fn encoding(&self) -> u32
    {
        self.1
    }

    pub fn assemble(parts: &[AsmInstrPart]) -> u32
    {
        let total_bits: u8 = parts.iter()
            .map(|p| p.bit_len())
            .sum();
        if total_bits != 32 {
            panic!("Failed to encode instruction, too may bits")
        }

        parts.iter().fold(0, |s, p| (s << p.shiftable_bit_len()) | p.encoding())
    }
}
