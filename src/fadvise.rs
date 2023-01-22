use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

use libc::{c_int, off_t, posix_fadvise, POSIX_FADV_DONTNEED};

pub fn flush(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    let len = meta.len();

    let res = unsafe {
        posix_fadvise(file.as_raw_fd() as c_int, 0, len as off_t, POSIX_FADV_DONTNEED)
    };

    if res != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
