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
    crate::{
        Guile,
        sys::{scm_dynwind_begin, scm_dynwind_end, scm_dynwind_unwind_handler},
    },
    std::{ffi::c_void, marker::PhantomData, pin::Pin, ptr},
};

#[repr(transparent)]
pub struct Dynwind<'gm> {
    _marker: PhantomData<&'gm ()>,
}
impl<'gm> Dynwind<'gm> {
    /// # Safety
    ///
    /// [Self::drop] must be ran, unless you abort.
    unsafe fn new(_: &'gm Guile) -> Self {
        unsafe {
            scm_dynwind_begin(0);
        }

        Self {
            _marker: PhantomData,
        }
    }
}
unsafe extern "C" fn cast_drop_in_place<T>(ptr: *mut c_void) {
    unsafe { ptr::drop_in_place::<T>(ptr.cast::<T>()) }
}
impl Dynwind<'_> {
    /// Protect a pointer in the current scope.
    ///
    /// If they are nested, the drop gets ran on the end of the current scope, not the end of this object.
    pub fn protect<'a, T>(&'a self, mut ptr: Pin<&'a mut T>) -> Pin<&'a mut T> {
        unsafe {
            scm_dynwind_unwind_handler(
                Some(cast_drop_in_place::<T>),
                ptr::from_mut(ptr.as_mut().get_unchecked_mut()).cast::<c_void>(),
                0,
            )
        };
        ptr
    }
}
impl<'gm> Dynwind<'gm> {
    /// Establish a scope where you can protect objects from guile unwinding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{collections::list::List, dynwind::Dynwind, Guile, with_guile};
    /// # use std::{pin::Pin, sync::atomic::{self, AtomicBool}};
    /// # #[cfg(not(miri))] {
    /// static DROPPED: AtomicBool = AtomicBool::new(false);
    /// struct MustDrop;
    /// impl Drop for MustDrop {
    ///     fn drop(&mut self) {
    ///         DROPPED.store(true, atomic::Ordering::Release);
    ///     }
    /// }
    /// fn test_drop<F>(f: F, unwind: bool)
    /// where
    ///     F: FnOnce(&Guile),
    /// {
    ///     DROPPED.store(false, atomic::Ordering::Release);
    ///     assert_eq!(
    ///         with_guile(|guile| {
    ///             Dynwind::scope(|wind| {
    ///                 let mut must_drop = MustDrop;
    ///                 wind.protect(Pin::new(&mut must_drop));
    ///                 f(guile)
    ///             }, guile)
    ///         })
    ///         .is_none(),
    ///         unwind
    ///     );
    ///     assert!(DROPPED.load(atomic::Ordering::Acquire));
    /// }
    /// test_drop(|guile| guile.misc_error(c"unknown", c"intentional error", List::<i32>::new(guile)), true);
    /// test_drop(|guile| {}, false);
    /// # }
    /// ```
    pub fn scope<F, O>(f: F, guile: &'gm Guile) -> O
    where
        F: FnOnce(&Self) -> O,
    {
        let dynwind = unsafe { Self::new(guile) };
        f(&dynwind)
    }
}
impl Drop for Dynwind<'_> {
    fn drop(&mut self) {
        // SAFETY: in order to get this object you must go through `Self::new` which creates a dynwind context
        unsafe {
            scm_dynwind_end();
        }
    }
}
