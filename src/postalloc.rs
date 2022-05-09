use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::io::Write;
use pgx::pg_sys::{self, SPI_palloc, pfree};
use pgx::log;

// SPI_palloc:
//
// Allocates using Postgres' "server programming interface".
// This means it uses a more durable memory context than the most transitory one: it pushes the allocation into the context that was used before the allocator.
// This is suitable for the "global allocator" but may not necessarily be the best one to use in general.
// 
// 
// SPI_pfree:

struct PostAlloc;

#[global_allocator]
static PALLOC: PostAlloc = PostAlloc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// static ALLOC_REPORT: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

unsafe impl GlobalAlloc for PostAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // #[cfg(feature = "std")]
        // {
            // let mut buf = [0u8; 256];
            // let counter = COUNTER.fetch_add(1, Ordering::AcqRel);
            // let mut msg = &mut buf;
            // (&mut msg[..]).write_fmt(format_args!("allocated/deallocated at least {counter} times."));
            // msg[255] = 0; // Guarantee null termination for the C writer.
            // pg_sys::write_stderr(msg as *const _ as *const i8);
        // }
        SPI_palloc(layout.size()).cast()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // if let Ok(&mut txt) = ALLOC_REPORT.get_mut() {
        // #[cfg(feature = "std")]
        // {
            // let mut buf = [0u8; 256];
            // let counter = COUNTER.fetch_add(1, Ordering::AcqRel);
            // let mut msg = &mut buf;
            // (&mut msg[..]).write_fmt(format_args!("allocated/deallocated at least {counter} times."));
            // msg[255] = 0; // Guarantee null termination for the C writer.
            // pg_sys::write_stderr(msg as *const _ as *const i8);
        // }
        pfree(ptr.cast())
    }
}
