use std::io::{Result, Write};

pub trait Writable
{
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<u64>;
}
