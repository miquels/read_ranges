use std::time::Instant;
use std::fs::File;
use std::io;
use read_ranges::{self, ReadRangeBuf, flush, willneed};

type ReadRangesFn = fn (&File, ReadRangeBuf) -> io::Result<ReadRangeBuf>;

fn bench(filename: &str, reader: ReadRangesFn, offset: &mut u64) -> io::Result<()> {
    let file = File::open(filename)?;
    flush(&file)?;

    for i in 0 .. 10 {
        let mut buf = ReadRangeBuf::default();
        for _ in 0 .. 200 {
            // buf.ranges.push((*offset, 640));
            // *offset += 68000;
            buf.ranges.push((*offset, 64000));
            *offset += 64000 + 4200;
        }
        let n = buf.ranges.len() - 1;
        let rlen = buf.ranges[n].0 + buf.ranges[n].1 as u64 - buf.ranges[0].0;
        willneed(&file, buf.ranges[0].0, rlen as usize)?;
        let buf = reader(&file, buf)?;
        println!("{}: {} (range: {})", i, buf.buf.len(), rlen);
        *offset += 6800000;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = std::env::args();
    let filename = args.nth(1).unwrap();
    let mut offset = 0;

    let start = Instant::now();
    bench(&filename, read_ranges::mmap::read_ranges, &mut offset)?;
    println!("aio::mmap::read_ranges: {:?}", start.elapsed());

    let start = Instant::now();
    bench(&filename, read_ranges::read_ranges, &mut offset)?;
    println!("aio::read_ranges: {:?}", start.elapsed());

    let start = Instant::now();
    bench(&filename, read_ranges::aio::read_ranges, &mut offset)?;
    println!("aio::aio::read_ranges: {:?}", start.elapsed());

    Ok(())
}
