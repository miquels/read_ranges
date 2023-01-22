use std::time::Instant;
use std::fs::File;
use std::io;
use read_ranges::{self, ReadRangeBuf, flush};

type ReadRangesFn = fn (&File, ReadRangeBuf) -> io::Result<ReadRangeBuf>;

fn bench(filename: &str, reader: ReadRangesFn) -> io::Result<()> {
    let file = File::open(filename)?;
    flush(&file)?;

    let mut offset = 0;

    for i in 0 .. 10 {
        let mut buf = ReadRangeBuf::default();
        for _ in 0 .. 200 {
            buf.ranges.push((offset, 640));
            offset += 68000;
        }
        let buf = reader(&file, buf)?;
        println!("{}: {}", i, buf.buf.len());
        offset += 6800000;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = std::env::args();
    let filename = args.nth(1).unwrap();

    let start = Instant::now();
    bench(&filename, read_ranges::read_ranges)?;
    println!("aio::read_ranges: {:?}", start.elapsed());

    let start = Instant::now();
    bench(&filename, read_ranges::mmap::read_ranges)?;
    println!("aio::mmap::read_ranges: {:?}", start.elapsed());

    let start = Instant::now();
    bench(&filename, read_ranges::aio::read_ranges)?;
    println!("aio::aio::read_ranges: {:?}", start.elapsed());

    Ok(())
}
