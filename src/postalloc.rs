use core::alloc::{GlobalAlloc, Layout};
use pgx::pg_sys;

struct PostAlloc;

#[global_allocator]
static PALLOC: PostAlloc = PostAlloc;

unsafe impl GlobalAlloc for PostAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        pg_sys::palloc(layout.size()).cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        pg_sys::pfree(ptr.cast());
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        pg_sys::repalloc(ptr.cast(), new_size).cast()
    }
}
