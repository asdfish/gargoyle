// gargoyle - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use {
    allocator_api2::alloc::{AllocError, Allocator, Layout},
    std::{ffi::c_void, num::NonZeroUsize, ptr::NonNull},
};

unsafe extern "C" {
    fn malloc(_: usize) -> *mut c_void;
    fn free(_: *mut c_void);
}

/// Allocator that allocates using [malloc] and [free].
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
