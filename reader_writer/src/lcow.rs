use std::fmt;
use std::ops::Deref;
use std::borrow::Borrow;

/// A lenient Cow
///
/// Similar to std::borrow::Cow, with an optional ToOwned/Clone bound on T.
pub enum LCow<'a, T: 'a>
{
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T: 'a> Clone for LCow<'a, T>
    where T: Clone
{
    fn clone(&self) -> Self
    {
        match *self {
            LCow::Borrowed(t) => LCow::Borrowed(t),
            LCow::Owned(ref t) => LCow::Owned(t.clone()),
        }
    }
}

impl<'a, T: 'a> LCow<'a, T>
    where T: Clone
{
    pub fn into_owned(self) -> T
    {
        match self {
            LCow::Borrowed(t) => t.clone(),
            LCow::Owned(t) => t,
        }
    }
}

impl<'a, T: 'a> Deref for LCow<'a, T>
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        match *self {
            LCow::Borrowed(t) => t,
            LCow::Owned(ref t) => t,
        }
    }
}

impl<'a, T: 'a> Borrow<T> for LCow<'a, T>
{
    fn borrow(&self) -> &T
    {
        match *self {
            LCow::Borrowed(r) => r,
            LCow::Owned(ref t) => t,
        }
    }
}

impl<'a, T> fmt::Debug for LCow<'a, T>
    where T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <T as fmt::Debug>::fmt(&self, f)
    }
}


// TODO: Other std traits?
