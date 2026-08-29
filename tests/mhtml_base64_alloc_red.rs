use anydoc::{Format, to_markdown_bytes};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;
static MAX_ALLOCATION: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MAX_ALLOCATION.fetch_max(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        MAX_ALLOCATION.fetch_max(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn base64_mime_parse_does_not_allocate_above_entry_limit_before_rejection() {
    const MAX_ENTRY_BYTES: usize = 128 * 1024 * 1024;
    const TRAILING_BYTES: usize = 180 * 1024 * 1024;

    let mut mhtml = Vec::with_capacity(TRAILING_BYTES + 1024);
    mhtml.extend_from_slice(
        b"MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nPHA+b2s8L3A+\r\n--b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n",
    );
    mhtml.resize(mhtml.len() + TRAILING_BYTES, b'x');
    mhtml.extend_from_slice(b"\r\n--b--\r\n");

    MAX_ALLOCATION.store(0, Ordering::Relaxed);
    let _ = to_markdown_bytes(&mhtml, Some(Format::Mhtml));
    let largest = MAX_ALLOCATION.load(Ordering::Relaxed);

    assert!(
        largest <= MAX_ENTRY_BYTES,
        "MHTML parsing allocated {largest} bytes before enforcing max_entry_bytes"
    );
}
