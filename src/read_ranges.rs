use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;

#[derive(Default)]
pub struct ReadRangeBuf {
    pub buf: Vec<u8>,
    pub ranges: Vec<(u64, usize)>,
}

pub(crate) struct Dropper(pub ReadRangeBuf);

impl Dropper {
    pub fn into_inner(mut self) -> ReadRangeBuf {
        std::mem::replace(&mut self.0, ReadRangeBuf::default())
    }
}

impl Drop for Dropper {
    fn drop(&mut self) {
        unsafe { self.0.buf.set_len(0) }
    }
}

pub fn read_ranges(file: &File, buf: ReadRangeBuf) -> io::Result<ReadRangeBuf> {
    let mut buf = Dropper(buf);

    // create a buffer as large as the sum of the ranges.
    let nbytes = buf.0.ranges.iter().map(|r| r.1).sum::<usize>();
    buf.0.buf.reserve_exact(nbytes);
    unsafe { buf.0.buf.set_len(nbytes) };

    let mut done = 0;
    for range in &buf.0.ranges {
        file.read_exact_at(&mut buf.0.buf[done .. done + range.1], range.0)?;
        done += range.1;
    }

    Ok(buf.into_inner())
}
