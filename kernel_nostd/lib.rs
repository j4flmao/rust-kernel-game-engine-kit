#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::panic::PanicInfo;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 64 * 1024;
struct Heap(UnsafeCell<[u8; HEAP_SIZE]>);
unsafe impl Sync for Heap {}
struct BumpAllocator { heap: Heap, cursor: AtomicUsize }
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(1);
        let size = layout.size();
        let result = self.cursor.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cursor| {
            let start = cursor.checked_add(align.saturating_sub(1))? & !(align - 1);
            start.checked_add(size).filter(|end| *end <= HEAP_SIZE)
        });
        let Ok(start) = result else { return core::ptr::null_mut(); };
        // SAFETY: fetch_update reserved a non-overlapping range in the heap.
        unsafe { (*self.heap.0.get()).as_mut_ptr().add(start) }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator { heap: Heap(UnsafeCell::new([0; HEAP_SIZE])), cursor: AtomicUsize::new(0) };

pub fn try_alloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    let layout = Layout::from_size_align(size, align).ok()?;
    let ptr = unsafe { ALLOCATOR.alloc(layout) };
    NonNull::new(ptr)
}

pub fn heap_remaining() -> usize { HEAP_SIZE.saturating_sub(ALLOCATOR.cursor.load(Ordering::Acquire)) }

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! { loop { core::hint::spin_loop(); } }

#[no_mangle]
pub extern "C" fn kernel_nostd_smoke() -> u32 { 1 }
