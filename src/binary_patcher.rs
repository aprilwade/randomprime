use std::{
    borrow::Cow,
    io,
    vec,
};

pub struct PatchedBinaryBuilder<'a>
{
    data: &'a [u8],
    patches: Vec<(usize, Cow<'a, [u8]>)>
}

impl<'a> PatchedBinaryBuilder<'a>
{
    pub fn new(data: &'a [u8]) -> PatchedBinaryBuilder<'a>
    {
        PatchedBinaryBuilder {
            data: data,
            patches: vec![],
        }
    }

    pub fn patch(mut self, start: usize, data: Cow<'a, [u8]>) -> Self
    {
        for patch in &self.patches {
            if (patch.0 < start && patch.0 + patch.1.len() > start) ||
               (start < patch.0 && start + data.len() > patch.0)
            {
                panic!("Overlapping patches")
            }
        }
        self.patches.push((start, data));
        self
    }

    pub fn build(mut self) -> PatchedBinary<'a>
    {
        let mut segments = vec![];
        self.patches.sort_by_key(|p| p.0);

        let mut pos = 0;
        for patch in self.patches {
            if pos < patch.0 {
                segments.push(Cow::Borrowed(&self.data[pos..patch.0]));
            }
            pos = patch.0 + patch.1.len();
            segments.push(patch.1);
        }
        if pos < self.data.len() {
            segments.push(Cow::Borrowed(&self.data[pos..]));
        }

        PatchedBinary {
            curr_segment: io::Cursor::new(Cow::Borrowed(&[])),
            segments: segments.into_iter(),
        }
    }
}

pub struct PatchedBinary<'a>
{
    curr_segment: io::Cursor<Cow<'a, [u8]>>,
    segments: vec::IntoIter<Cow<'a, [u8]>>,
}

impl<'a> io::Read for PatchedBinary<'a>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let mut total_bytes_written = 0;
        loop {
            let offset = self.curr_segment.read(&mut buf[total_bytes_written..])?;
            total_bytes_written += offset;
            // Have we completely filed the buffer yet?
            if total_bytes_written >= buf.len() {
                break;
            }


            if let Some(seg) = self.segments.next() {
                self.curr_segment = io::Cursor::new(seg);
            } else {
                self.curr_segment = io::Cursor::new(Cow::Borrowed(&[]));
                break
            };
        }
        return Ok(total_bytes_written);
    }
}
