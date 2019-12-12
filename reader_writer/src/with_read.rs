use std::fmt;
use std::io::{self, Read};

/// An object-safe trait for objects that can be Read multiple types
pub trait WithRead: fmt::Debug
{
    fn len(&self) -> usize;
    fn boxed<'r>(&self) -> Box<dyn WithRead + 'r>
        where Self: 'r;
    fn with_read(&self, f: &mut dyn FnMut(&mut dyn Read) -> io::Result<u64>) -> io::Result<u64>;
}

impl<'r> Clone for Box<dyn WithRead + 'r>
{
    fn clone(&self) -> Self
    {
        self.boxed()
    }
}

impl<T> WithRead for T
    where T: AsRef<[u8]> + fmt::Debug + Clone
{
    fn len(&self) -> usize
    {
        self.as_ref().len()
    }

    fn boxed<'r>(&self) -> Box<dyn WithRead + 'r>
        where Self: 'r
    {
        Box::new(self.clone())
    }

    fn with_read(&self, f: &mut dyn FnMut(&mut dyn Read) -> io::Result<u64>) -> io::Result<u64>
    {
        f(&mut io::Cursor::new(self.as_ref()))
    }

}
