use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init(heap_start: usize, heap_size: usize) {
    crate::serial::serial_write_line("  allocator lock...");
    unsafe {
        let mut lock = ALLOCATOR.lock();
        crate::serial::serial_write_line("  allocator lock acquired");
        lock.init(heap_start as *mut u8, heap_size);
        crate::serial::serial_write_line("  allocator init done");
    }
    crate::serial::serial_write_line("  allocator init complete");
}
