use std::fs::File;
use std::io::{self, Error as IoError};
use std::ops::IndexMut;
use std::os::fd::AsRawFd;

use libc::{
    __error,
    EAGAIN,
    EINTR,
    EIO,
    LIO_READ,
    LIO_WAIT,
    aio_return,
    lio_listio,
    c_int,
    sigevent as sigevent_t,
    aiocb as aiocb_t,
    off_t,
    size_t,
    c_void,
};

use super::ReadRangeBuf;
use super::Dropper;

const MAX_REQUESTS: usize = 1_usize;

pub fn read_ranges(file: &File, buf: ReadRangeBuf) -> io::Result<ReadRangeBuf> {
    let mut buf = Dropper(buf);
    let mut aiocbs = Vec::new();
    let nreq = buf.0.ranges.len();

    // create a buffer as large as the sum of the ranges.
    let nbytes = buf.0.ranges.iter().map(|r| r.1).sum::<usize>();
    buf.0.buf.reserve_exact(nbytes);
    unsafe { buf.0.buf.set_len(nbytes) };
    let bufptr = &mut buf.0.buf[0] as *mut u8 as u64;
    let mut bufoff = 0;

    let mut done = 0;
    while done < nreq {

        // Create an array of requests.
        let todo = std::cmp::min(nreq - done, MAX_REQUESTS);
        aiocbs.clear();
        aiocbs.reserve(todo);

        for range in &buf.0.ranges[done .. done + todo] {
            // create an aiocb. why doesn't aiocb implement Default?
            // SAFETY: only contains pointers and values, no references.
            let mut aiocb: aiocb_t = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

            aiocb.aio_fildes = file.as_raw_fd() as c_int;
            aiocb.aio_offset = range.0 as off_t;
            aiocb.aio_buf = (bufptr + bufoff) as *mut c_void;
            aiocb.aio_nbytes = range.1 as size_t;
            aiocb.aio_reqprio = 0;
            aiocb.aio_lio_opcode = LIO_READ;

            aiocbs.push(aiocb);
            bufoff += range.1 as u64;
        }

        // We need an array of pointers to aiocbs.
        let nr = aiocbs.len();
        let mut aiocb_ptrs = aiocbs.iter_mut().collect::<Vec<_>>();

        // Call lio_listio in blocking mode.
        let res = unsafe {
            lio_listio(
                LIO_WAIT,
                aiocb_ptrs.as_mut_ptr() as *const *mut aiocb_t,
                nr as c_int,
                0usize as *mut sigevent_t
            )
        };

        // Check for submission errors.
        let mut errno = 0;
        if res != 0 {
            errno = unsafe { *__error() };
            if errno == EAGAIN || errno == EINTR || errno == EIO {
                return Err(IoError::from_raw_os_error(errno));
            }
        }

        // Check for result errors.
        for idx in 0 .. nr {
            let err = unsafe { aio_return(*aiocb_ptrs.index_mut(idx) as *mut aiocb_t) };
            if err == 0 {
                if buf.0.ranges[idx+done].1 as size_t != aiocb_ptrs[idx].aio_nbytes {
                    errno = EIO;
                }
            } else {
                errno = unsafe { *__error() };
            }
        }

        // Return error, if any.
        if errno != 0 {
            return Err(IoError::from_raw_os_error(errno));
        }

        aiocb_ptrs.clear();
        done += todo;
    }

    Ok(buf.into_inner())
}
