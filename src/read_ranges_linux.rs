use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Error as IoError, ErrorKind};
use std::os::fd::AsRawFd;

use nc::call;
use nc::types::{
    IOCB_CMD_PREAD,
    aio_context_t,
    io_event_t,
    iocb_t,
    timespec_t,
    time_t,
};

use super::{ReadRangeBuf, Dropper};

struct AioContext(aio_context_t);

// A lazily initialized thread-local AioContext.
//
// By using a context per thread, we can be sure that we're the only ones
// submitting requests to the context, so io_getevents() returns just
// the results for those requests and we don't have to de-multiplex.
thread_local!(static AIOCTX: RefCell<Option<io::Result<AioContext>>> = RefCell::new(None));

pub fn read_ranges(file: &File, buf: ReadRangeBuf) -> io::Result<ReadRangeBuf> {
    AIOCTX.with(|ctx| {
        match ctx.borrow_mut().get_or_insert_with(|| AioContext::new(256)) {
            Ok(ctx) => ctx.read_ranges(file, buf),
            Err(e) => Err(io::Error::from_raw_os_error(e.raw_os_error().unwrap())),
        }
    })
}

impl AioContext {
    fn new(nr_events: u32) -> io::Result<AioContext> {
        unsafe {
            let mut ctx_id = 0;
            call::io_setup(nr_events, &mut ctx_id).map_err(IoError::from_raw_os_error)?;
            Ok(AioContext(ctx_id))
        }
    }

    fn read_ranges(&mut self, file: &File, buf: ReadRangeBuf) -> io::Result<ReadRangeBuf> {
        let mut buf = Dropper(buf);
        let mut iocbs = Vec::new();

        let nbytes = buf.0.ranges.iter().map(|r| r.1).sum::<usize>();
        buf.0.buf.reserve_exact(nbytes);
        unsafe { buf.0.buf.set_len(nbytes) };
        let bufptr = &mut buf.0.buf[0] as *mut u8 as u64;
        let mut bufoff = 0;

        for range in &buf.0.ranges {
            iocbs.push(iocb_t {
                aio_data: 0,
                aio_key: 0,
                aio_rw_flags: 0,
                aio_lio_opcode: IOCB_CMD_PREAD as u16,
                aio_reqprio: 0,
                aio_fildes: file.as_raw_fd() as u32,
                aio_buf: bufptr + bufoff,
                aio_nbytes: range.1 as u64,
                aio_offset: range.0 as i64,
                aio_reserved2: 0,
                aio_flags: 0,
                aio_resfd: 0,
            });
            bufoff += range.1 as u64;
        }
        let nr = iocbs.len();
        let mut iocb_ptrs = iocbs.iter_mut().collect::<Vec<_>>();
        let mut events = Vec::new();
        let mut done = 0;

        while done != nr {

            let nevs = {
                // We need this because io_submit()'s 'iocb' arg is defined as
                // &mut iocb_t, while it _should_ be &mut iocb_t[].
                let iocb_ptrs = iocb_ptrs[done..].as_mut_ptr() as u64;
                let iocb_ptrs = unsafe { &mut *(iocb_ptrs as *mut iocb_t) };

                let todo = (nr - done) as isize;
                match unsafe { call::io_submit(self.0, todo, iocb_ptrs) } {
                    Ok(n) => n as isize,
                    Err(e) => return Err(IoError::from_raw_os_error(e)),
                }
            };

            events.resize_with(nevs as usize, io_event_t::default);
            let mut tmout = timespec_t {
                tv_sec: time_t::MAX / 2,
                tv_nsec: 0,
            };
            match unsafe { call::io_getevents(self.0, nevs, nevs, &mut events[0], &mut tmout) } {
                Ok(_) => {},
                Err(e) => return Err(IoError::from_raw_os_error(e)),
            }

            for ev in &events {
                let iocb = iocb_ptrs[done..]
                    .iter()
                    .find(|&iocb| *iocb as *const iocb_t as u64 == ev.obj);
                let iocb = match iocb {
                    Some(iocb) => iocb,
                    None => return Err(IoError::new(ErrorKind::Other, "event for unknown iocb")),
                };
                if ev.res < 0 {
                    return Err(IoError::from_raw_os_error(-ev.res as i32));
                }
                if ev.res as u64 != iocb.aio_nbytes {
                    return Err(IoError::new(ErrorKind::UnexpectedEof, "short read"));
                }
            }

            done += nevs as usize;
        }

        Ok(buf.into_inner())
    }
}

impl Drop for AioContext {
    fn drop(&mut self) {
        unsafe { let _ = call::io_destroy(self.0); }
    }
}
