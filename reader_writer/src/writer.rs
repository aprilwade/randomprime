use std::io::{Result, Write};

pub trait Writable
{
    fn write<W: Write>(&self, writer: &mut W) -> Result<()>;
}
