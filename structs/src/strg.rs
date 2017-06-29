
use reader_writer::{Dap, FourCC, IteratorArray, LCow, LazyArray, LazyUtf16beStr, Readable, RoArray, RoArrayIter,};

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Strg<'a>
    {
        #[expect = 0x87654321]
        magic: u32,
        #[expect = 0]
        version: u32,

        #[derivable = string_tables.len() as u32]
        lang_count: u32,
        // TODO: It might be nice to have an assert that all the tables have the same length
        #[derivable = string_tables.iter().next().unwrap().strings.len() as u32]
        string_count: u32,

        #[derivable: Dap<_, _> = string_tables.iter()
            .scan(0usize, &|sum: &mut usize, t: LCow<StrgStringTable>| {
                let r = StrgLang { lang: t.lang, offset: *sum as u32, };
                *sum += t.size();
                Some(r)
            }).into()]
        langs: RoArray<'a, StrgLang> = (lang_count as usize, ()),
        string_tables: IteratorArray<'a, StrgStringTable<'a>, StrgLangIter<'a>>
            = StrgLangIter(string_count as usize, langs.iter()),
    }
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct StrgLangIter<'a>(usize, RoArrayIter<'a, StrgLang>);
impl<'a> Iterator for StrgLangIter<'a>
{
    type Item = (usize, FourCC);
    fn next(&mut self) -> Option<Self::Item>
    {
        self.1.next().map(|i| (self.0, i.lang))
    }
}
impl<'a> ExactSizeIterator for StrgLangIter<'a>
{
    fn len(&self) -> usize
    {
        self.1.len()
    }
}



auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    struct StrgLang
    {
        lang: FourCC,
        offset: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct StrgStringTable<'a>
    {
        #[args]
        (string_count, lang): (usize, FourCC),

        #[literal]
        lang: FourCC = lang,

        #[derivable = (strings.len() * 4 + strings.iter()
            .map(&|i: LCow<LazyUtf16beStr>| i.size())
            .sum::<usize>()) as u32]
        _size: u32,

        #[derivable: Dap<_, _> = strings.iter()
            .scan(strings.len() as u32 * 4, &|st: &mut u32, i: LCow<LazyUtf16beStr>| {
                let r = *st;
                *st += i.size() as u32;
                Some(r)
            }).into()]
        _offsets: RoArray<'a, u32> = (string_count, ()),
        strings: LazyArray<'a, LazyUtf16beStr<'a>> = (string_count, ()),
    }
}
