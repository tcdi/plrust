use core::alloc::{GlobalAlloc, Layout};
use pgx::pg_sys::{self, SPI_palloc, SPI_repalloc, SPI_pfree};
use pgx::log;

struct PostAlloc;

#[global_allocator]
static PALLOC: PostAlloc = PostAlloc;

unsafe impl GlobalAlloc for PostAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let msg = "attempt to emit alloc message from custom allocator";
        pg_sys::write_stderr(msg as *const _ as *const i8);
        SPI_palloc(layout.size()).cast()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let msg = "attempt to emit dealloc message from custom allocator";
        pg_sys::write_stderr(msg as *const _ as *const i8);
        SPI_pfree(ptr.cast())
    }
}
