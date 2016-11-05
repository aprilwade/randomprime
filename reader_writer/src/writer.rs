use std::io::Write;

pub trait Writable
{
    fn write<W: Write>(&self, writer: &mut W);
}
