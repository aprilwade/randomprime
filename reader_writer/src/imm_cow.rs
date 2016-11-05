use std::fmt;
use std::ops::Deref;
use std::borrow::Borrow;

/// An immutable Cow.
pub struct ImmCow<'a, T: 'a>(ImmCow_<'a, T>);

enum ImmCow_<'a, T: 'a>
{
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T: 'a> ImmCow<'a, T>
{
    pub fn new_owned(t: T) -> ImmCow<'a, T>
    {
        ImmCow(ImmCow_::Owned(t))
    }

    pub fn new_borrowed(t: &'a T) -> ImmCow<'a, T>
    {
        ImmCow(ImmCow_::Borrowed(t))
    }

    // Needed since pattern matching isn't allowed.
    pub fn try_get_borrowed(&self) -> Option<&'a T>
    {
        match self.0 {
            ImmCow_::Borrowed(t) => Some(t),
            _ => None
        }
    }
}

impl<'a, T: 'a> Deref for ImmCow<'a, T>
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        match self.0 {
            ImmCow_::Borrowed(t) => t,
            ImmCow_::Owned(ref t) => t,
        }
    }
}

impl<'a, T: 'a> Borrow<T> for ImmCow<'a, T>
{
    fn borrow(&self) -> &T
    {
        match self.0 {
            ImmCow_::Borrowed(r) => r,
            ImmCow_::Owned(ref t) => t,
        }
    }
}

impl<'a, T> fmt::Debug for ImmCow<'a, T>
    where T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <T as fmt::Debug>::fmt(&self, f)
    }
}


// TODO: Other std traits?
