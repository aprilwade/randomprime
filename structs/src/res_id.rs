use reader_writer::{FourCC, Readable, Reader, Writable};

use std::convert::TryFrom;
use std::fmt;
use std::io;
use std::marker::PhantomData;

pub trait ResIdKind {
    const FOURCC: FourCC;
}

macro_rules! decl_res_id_kind {
    ($($id:ident $e:expr,)*) => {
        $(
            #[allow(non_camel_case)]
            #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
            pub struct $id;
            impl ResIdKind for $id
            {
                const FOURCC: FourCC = FourCC::from_bytes(&$e);
            }
        )*
    };
}

decl_res_id_kind! {
    ANCS b"ANCS",
    ANIM b"ANIM",
    AGSC b"AGSC",
    AFSM b"AFSM",
    CINF b"CINF",
    CMDL b"CMDL",
    CSKR b"CSKR",
    DCLN b"DCLN",
    ELSC b"ELSC",
    EVNT b"EVNT",
    FONT b"FONT",
    FRME b"FRME",
    MAPA b"MAPA",
    MREA b"MREA",
    MLVL b"MLVL",
    PART b"PART",
    SAVW b"SAVW",
    SCAN b"SCAN",
    SHWC b"SHWC",
    STRG b"STRG",
    TXTR b"TXTR",
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResId<K>(u32, PhantomData<K>);

impl<K> ResId<K>
{
    pub const fn new(i: u32) -> Self
    {
        ResId(i, PhantomData)
    }

    pub const fn invalid() -> Self
    {
        Self::new(0xFFFFFFFF)
    }

    pub const fn to_u32(self) -> u32
    {
        self.0
    }
}


impl<K> Into<u32> for ResId<K>
{
    fn into(self) -> u32
    {
        self.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WrongFourCCError;

impl<K: ResIdKind> TryFrom<resource_info_table::ResourceInfo> for ResId<K>
{
    type Error = WrongFourCCError;
    fn try_from(res_info: resource_info_table::ResourceInfo) -> Result<Self, Self::Error>
    {
        if K::FOURCC != res_info.fourcc {
            Err(WrongFourCCError)
        } else {
            Ok(ResId(res_info.res_id, PhantomData))
        }
    }
}

impl<K: ResIdKind> Into<(u32, FourCC)> for ResId<K>
{
    fn into(self) -> (u32, FourCC)
    {
        (self.0, K::FOURCC)
    }
}

impl<K: ResIdKind> Into<crate::Dependency> for ResId<K>
{
    fn into(self) -> crate::Dependency
    {
        crate::Dependency {
            asset_id: self.0,
            asset_type: K::FOURCC,
        }
    }
}

impl<'r, K> Readable<'r> for ResId<K>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        ResId(reader.read(()), PhantomData)
    }

    fn fixed_size() -> Option<usize>
    {
        <u32 as Readable<'r>>::fixed_size()
    }
}

impl<K> Writable for ResId<K>
{
    fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<u64>
    {
        self.0.write_to(w)
    }
}

impl<K: ResIdKind> fmt::Debug for ResId<K>
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result
    {
        write!(formatter, "ResId<{:?}>({:08x})", K::FOURCC, self.0)
    }
}

impl<K> PartialEq<u32> for ResId<K>
{
    fn eq(&self, other: &u32) -> bool
    {
        &self.0 == other
    }
}
