#[cfg(target_os = "freebsd")]
mod read_ranges_freebsd;
#[cfg(target_os = "freebsd")]
pub mod aio {
    pub use super::read_ranges_freebsd::*;
}

#[cfg(target_os = "linux")]
mod read_ranges_linux;
#[cfg(target_os = "linux")]
pub mod aio {
    pub use super::read_ranges_linux::*;
}

mod read_ranges_mmap;
pub mod mmap {
    pub use super::read_ranges_mmap::*;
}

mod read_ranges;
pub use read_ranges::*;
