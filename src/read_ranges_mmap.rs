use std::fs::File;
use std::io;
use memmap2::MmapOptions;
use super::ReadRangeBuf;
use super::Dropper;

pub fn read_ranges(file: &File, buf: ReadRangeBuf) -> io::Result<ReadRangeBuf> {

    let mut buf = buf;
    if buf.ranges.len() == 0 {
        buf.buf.clear();
        return Ok(buf);
    }
    let base = buf.ranges[0].0;

    let last = buf.ranges.len() - 1;
    let mmap = unsafe {
        MmapOptions::new()
            .offset(base)
            .len((buf.ranges[last].0 - base) as usize + buf.ranges[last].1)
            .map(file)?
    };

    // create a buffer as large as the sum of the ranges.
    let nbytes = buf.ranges.iter().map(|r| r.1).sum::<usize>();
    buf.buf.reserve_exact(nbytes);
    let mut buf = Dropper(buf);
    unsafe { buf.0.buf.set_len(nbytes) };

    // copy.
    let mut done = 0;
    for range in &buf.0.ranges {
        let src = &mmap[(range.0 - base) as usize..][..range.1];
        let dst = &mut buf.0.buf[done .. done + range.1];
        dst.copy_from_slice(src);
        done += range.1;
    }

    Ok(buf.into_inner())
}
