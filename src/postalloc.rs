use core::alloc::{GlobalAlloc, Layout};
use pgx::pg_sys::{SPI_palloc, SPI_repalloc, SPI_pfree};
use pgx::log;

struct PostAlloc;

#[global_allocator]
static PALLOC: PostAlloc = PostAlloc;

unsafe impl GlobalAlloc for PostAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        log!("allocated using custom allocator!");
        SPI_palloc(layout.size()).cast()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        log!("deallocated using custom allocator!");
        SPI_pfree(ptr.cast())
    }
}
