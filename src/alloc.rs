use {
    crate::sys::{free, malloc},
    allocator_api2::alloc::{AllocError, Allocator, Layout},
    std::{num::NonZeroUsize, ptr::NonNull},
};

/// Allocator that allocates using [malloc] and [free] from [crate::sys].
///
/// You should use this over the global allocator since that may be changed.
pub struct CAllocator;

unsafe impl Allocator for CAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();
        NonZeroUsize::new(size)
            .ok_or(AllocError)
            // SAFETY: bytes is not zero
            .map(|bytes| unsafe { malloc(bytes.get()) })
            .and_then(|ptr| NonNull::new(ptr).ok_or(AllocError))
            .map(NonNull::cast)
            .map(|ptr| NonNull::slice_from_raw_parts(ptr, size))
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, _: Layout) {
        unsafe {
            free(ptr.as_ptr().cast());
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, allocator_api2::vec::Vec};

    #[test]
    fn empty() {
        let _vec: Vec<u32, CAllocator> = Vec::new_in(CAllocator);
        let _vec: Vec<u32, CAllocator> = Vec::with_capacity_in(0, CAllocator);

        let mut vec = Vec::new_in(CAllocator);
        vec.push(1);
    }

    #[test]
    fn vec_alloc() {
        let mut vec: Vec<u32, CAllocator> = Vec::with_capacity_in(2, CAllocator);
        vec.push(1);
        vec.push(2);

        assert_eq!(vec, [1, 2]);
    }
}
