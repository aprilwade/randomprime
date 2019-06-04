#[macro_use] extern crate auto_struct_macros;
extern crate reader_writer;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone)]
struct FixedSizeTest
{
    i: u32,
    m: u32,
    y: u16,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone)]
struct SizeTest<'r>
{
    #[auto_struct(args)]
    i: u32,

    #[auto_struct(expect = 0xFFFF)]
    x: u16,

    #[auto_struct(init = (i as usize, ()))]
    data: reader_writer::RoArray<'r, u8>,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone)]
struct PaddingTest
{
    i: u32,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    j: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone)]
struct DeriveFromIteratorTest<'r>
{
    #[auto_struct(derive = array.len() as u32)]
    count: u32,

    #[auto_struct(init = (count as usize, ()))]
    #[auto_struct(derive_from_iter = array.iter().map(|i| i.data.len() as u32))]
    args: reader_writer::RoArray<'r, u32>,

    #[auto_struct(init = args.iter())]
    array: reader_writer::IteratorArray<
        'r,
        SizeTest<'r>,
        reader_writer::RoArrayIter<'r, u32>,
    >,
}


#[test]
fn test_size()
{
    use reader_writer::Readable;
    let data = [0xFFu8; 32];
    let mut reader = reader_writer::Reader::new(&data[..]);
    let size_test: SizeTest = reader.read(8);
    assert_eq!(size_test.size(), 10);
}

#[test]
fn test_fixed_size()
{
    use reader_writer::Readable;
    assert_eq!(FixedSizeTest::fixed_size(), Some(10));
    assert_eq!(SizeTest::fixed_size(), None);
}

#[test]
fn test_padding()
{
    use reader_writer::Readable;
    let data = [0xFFu8; 36];
    let mut reader = reader_writer::Reader::new(&data[..]);
    let padding_test: PaddingTest = reader.read(());
    assert_eq!(padding_test.size(), 36);
}
