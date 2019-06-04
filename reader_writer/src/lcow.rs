use std::{
    fmt,
    ops::Deref,
    borrow::Borrow,
};

/// A lenient Cow
///
/// Similar to std::borrow::Cow, with an optional ToOwned/Clone bound on T.
pub enum LCow<'r, T>
{
    Borrowed(&'r T),
    Owned(T),
}

impl<'r, T> Clone for LCow<'r, T>
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

impl<'r, T> LCow<'r, T>
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

impl<'r, T> Deref for LCow<'r, T>
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

impl<'r, T> Borrow<T> for LCow<'r, T>
{
    fn borrow(&self) -> &T
    {
        match *self {
            LCow::Borrowed(r) => r,
            LCow::Owned(ref t) => t,
        }
    }
}

impl<'r, T> fmt::Debug for LCow<'r, T>
    where T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <T as fmt::Debug>::fmt(&self, f)
    }
}


// TODO: Other std traits?
