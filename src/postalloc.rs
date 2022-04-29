use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::io::Write;
use pgx::pg_sys::{self, SPI_palloc, SPI_pfree};
use pgx::log;

struct PostAlloc;

#[global_allocator]
static PALLOC: PostAlloc = PostAlloc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for PostAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut buf = [0u8; 256];
        let counter = COUNTER.fetch_add(1, Ordering::AcqRel);
        let mut msg = &mut buf;
        (&mut msg[..]).write_fmt(format_args!("allocated/deallocated at least {counter} times."));
        msg[255] = 0; // Guarantee null termination for the C writer.
        pg_sys::write_stderr(msg as *const _ as *const i8);
        SPI_palloc(layout.size()).cast()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut buf = [0u8; 256];
        let counter = COUNTER.fetch_add(1, Ordering::AcqRel);
        let mut msg = &mut buf;
        (&mut msg[..]).write_fmt(format_args!("allocated/deallocated at least {counter} times."));
        msg[255] = 0; // Guarantee null termination for the C writer.
        pg_sys::write_stderr(msg as *const _ as *const i8);
        SPI_pfree(ptr.cast())
    }
}
