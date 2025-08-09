// garguile - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Implementations of [Allocator].

use {
    crate::{Guile, sys::scm_gc_malloc},
    allocator_api2::alloc::{AllocError, Allocator, Layout},
    std::{
        ffi::{CStr, c_void},
        marker::PhantomData,
        ptr::NonNull,
    },
};

unsafe extern "C" {
    fn malloc(_: usize) -> *mut c_void;
    fn free(_: *mut c_void);
}

/// Allocator that uses `malloc` and `free`.
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

/// Allocator that uses the guile garbage collector.
#[derive(Clone, Copy)]
pub struct GcAllocator<'gm, 'a> {
    purpose: &'a CStr,
    _marker: PhantomData<&'gm ()>,
}
impl<'gm, 'a> GcAllocator<'gm, 'a> {
    /// Create a new allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use allocator_api2::boxed::Box;
    /// # use garguile::{alloc::GcAllocator, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let allocator = GcAllocator::new(c"box", guile);
    ///     Box::new_in(10, allocator);
    /// }).unwrap();
    /// ```
    pub fn new(purpose: &'a CStr, _: &'gm Guile) -> Self {
        Self {
            purpose,
            _marker: PhantomData,
        }
    }
}
unsafe impl Allocator for GcAllocator<'_, '_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();

        NonNull::new(unsafe { scm_gc_malloc(size, self.purpose.as_ptr()) }.cast::<u8>())
            .map(|ptr| NonNull::slice_from_raw_parts(ptr, size))
            .ok_or(AllocError)
    }
    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}
// impl<'gm> From<&'gm Guile> for GcAllocator<'gm> {
//     fn from(guile: &'gm Guile) -> Self {
//         Self { _guile: guile }
//     }
// }

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
