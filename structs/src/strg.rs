use auto_struct_macros::auto_struct;

use reader_writer::{
    FourCC, IteratorArray, LCow, LazyArray, LazyUtf16beStr, Readable, RoArray,
    RoArrayIter,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Strg<'r>
{
    #[auto_struct(expect = 0x87654321)]
    magic: u32,
    #[auto_struct(expect = 0)]
    version: u32,

    #[auto_struct(derive = string_tables.len() as u32)]
    lang_count: u32,
    // TODO: It might be nice to have an assert that all the tables have the same length
    #[auto_struct(derive = string_tables.iter().next().unwrap().strings.len() as u32)]
    string_count: u32,

    #[auto_struct(derive_from_iter = string_tables.iter()
        .scan(0usize, &|sum: &mut usize, t: LCow<StrgStringTable>| {
            let r = StrgLang { lang: t.lang, offset: *sum as u32, };
            *sum += t.size();
            Some(r)
        }))]
    #[auto_struct(init = (lang_count as usize, ()))]
    langs: RoArray<'r, StrgLang>,
    #[auto_struct(init = StrgLangIter(string_count as usize, langs.iter()))]
    pub string_tables: IteratorArray<'r, StrgStringTable<'r>, StrgLangIter<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl<'r> Strg<'r>
{
    pub fn from_strings(strings: Vec<String>) -> Strg<'r>
    {
        Strg {
            string_tables: vec![StrgStringTable {
                lang: b"ENGL".into(),
                strings: strings.into_iter().map(|i| i.into()).collect::<Vec<_>>().into(),
            }].into(),
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct StrgLangIter<'r>(usize, RoArrayIter<'r, StrgLang>);
impl<'r> Iterator for StrgLangIter<'r>
{
    type Item = (usize, FourCC);
    fn next(&mut self) -> Option<Self::Item>
    {
        self.1.next().map(|i| (self.0, i.lang))
    }
}
impl<'r> ExactSizeIterator for StrgLangIter<'r>
{
    fn len(&self) -> usize
    {
        self.1.len()
    }
}



#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
struct StrgLang
{
    pub lang: FourCC,
    pub offset: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct StrgStringTable<'r>
{
    #[auto_struct(args = (string_count, lang))]
    _args: (usize, FourCC),

    #[auto_struct(literal = lang)]
    pub lang: FourCC,

    #[auto_struct(derive = (strings.len() * 4 + strings.iter()
        .map(&|i: LCow<LazyUtf16beStr>| i.size())
        .sum::<usize>()) as u32)]
    _size: u32,

    #[auto_struct(derive_from_iter = strings.iter()
        .scan(strings.len() as u32 * 4, &|st: &mut u32, i: LCow<LazyUtf16beStr>| {
            let r = *st;
            *st += i.size() as u32;
            Some(r)
        }))]
    #[auto_struct(init = (string_count, ()))]
    _offsets: RoArray<'r, u32>,
    #[auto_struct(init = (string_count, ()))]
    pub strings: LazyArray<'r, LazyUtf16beStr<'r>>,
}
