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

//! Ensure calls to drop in case of stack unwinding.

use {
    crate::{
        Guile,
        sys::{scm_dynwind_begin, scm_dynwind_end, scm_dynwind_unwind_handler},
    },
    std::{ffi::c_void, marker::PhantomData, pin::Pin, ptr},
};

/// Raii guard for dynamic wind scopes.
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
    /// # use garguile::{collections::list::List, dynwind::Dynwind, symbol::Symbol, Guile, with_guile};
    /// # use std::{pin::Pin, sync::atomic::{self, AtomicBool}};
    /// # #[cfg(not(miri))] {
    /// static DROPPED: AtomicBool = AtomicBool::new(false);
    /// struct MustDrop;
    /// impl Drop for MustDrop {
    ///     fn drop(&mut self) {
    ///         assert!(!DROPPED.load(atomic::Ordering::Acquire));
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
    /// test_drop(|guile| guile.throw(Symbol::from_str("intentional-error", guile), List::<i32>::new(guile)), true);
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
