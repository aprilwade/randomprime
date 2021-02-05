// use std::io::{self, Write};

pub trait Writer {
    type Error;

    fn write_bytes(&mut self, _: &[u8]) -> Result<(), Self::Error>;

    fn write_bool(&mut self, _: bool) -> Result<(), Self::Error>;
    fn write_u8(&mut self, _: u8) -> Result<(), Self::Error>;
    fn write_i8(&mut self, _: i8) -> Result<(), Self::Error>;
    fn write_u16(&mut self, _: u16) -> Result<(), Self::Error>;
    fn write_i16(&mut self, _: i16) -> Result<(), Self::Error>;
    fn write_u32(&mut self, _: u32) -> Result<(), Self::Error>;
    fn write_i32(&mut self, _: i32) -> Result<(), Self::Error>;
    fn write_u64(&mut self, _: u64) -> Result<(), Self::Error>;
    fn write_i64(&mut self, _: i64) -> Result<(), Self::Error>;
    fn write_f32(&mut self, _: f32) -> Result<(), Self::Error>;
    fn write_f64(&mut self, _: f64) -> Result<(), Self::Error>;
}

pub trait Writable<W: Writer>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>;
}

// pub trait Writable
// {
//     fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<u64>;
// }
