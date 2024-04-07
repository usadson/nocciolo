use core::marker::PhantomData;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::VirtAddr;
use crate::allocator::Locked;

lazy_static! {
    static ref ALLOCATOR: Locked<PageAllocatorImpl> = Locked::new(PageAllocatorImpl::new());
}

pub struct PageAllocator {
    _data: PhantomData<()>,
}

impl PageAllocator {
    pub fn allocate() -> VirtAddr {
        Self::allocate_n(1)
    }

    pub fn allocate_n(n: usize) -> VirtAddr {
        assert_ne!(n, 0);

        let size = n as u64 * 4096;

        let mut allocator = ALLOCATOR.lock();
        let addr = allocator.addr;
        allocator.addr += size;

        addr
    }
}

struct PageAllocatorImpl {
    addr: VirtAddr,
}

impl PageAllocatorImpl {
    pub fn new() -> Self {
        Self {
            addr: VirtAddr::new_truncate(0x1_000_000_000),
        }
    }
}
