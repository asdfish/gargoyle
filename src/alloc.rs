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
    std::{ffi::c_void, ptr::NonNull},
};

unsafe extern "C" {
    pub fn malloc(_: usize) -> *mut c_void;
    pub fn free(_: *mut c_void);
}

/// Allocator that uses [malloc] and [free].
pub struct CAllocator;

unsafe impl Allocator for CAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match layout.size() {
            0 => None,
            bytes => NonNull::new(unsafe { malloc(bytes) }.cast::<u8>())
                .map(|ptr| NonNull::slice_from_raw_parts(ptr, bytes)),
        }
        .ok_or(AllocError)
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, _: Layout) {
        unsafe { free(ptr.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, allocator_api2::vec::Vec};

    #[test]
    fn c_allocator() {
        let mut vec = Vec::new_in(CAllocator);
        (0..3).for_each(|i| vec.push(i));
        assert_eq!(vec, [0, 1, 2]);
    }
}
