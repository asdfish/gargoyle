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
        collections::list::List,
        scm::ToScm,
        symbol::Symbol,
        sys::{SCM, scm_unused_struct},
    },
    std::{
        ffi::CStr,
        sync::{
            LazyLock,
            atomic::{self, AtomicPtr},
        },
    },
};

/// Return a list comprised of `'(data)`
///
/// This is only exported for the `ForeignObject` derive macro.
///
/// # Safety
///
/// You must be in guile mode.
#[doc(hidden)]
pub unsafe fn slots() -> SCM {
    static SYMBOL: LazyLock<AtomicPtr<scm_unused_struct>> = LazyLock::new(|| {
        let guile = unsafe { Guile::new_unchecked_ref() };
        List::from_iter([Symbol::from_str("data", guile)], guile)
            .to_scm(guile)
            .as_ptr()
            .into()
    });

    SYMBOL.load(atomic::Ordering::Acquire)
}

pub trait ForeignObject: Copy {
    const TYPE_NAME: &CStr;

    /// Create a type tag.
    ///
    /// # Safety
    ///
    /// Only call in guile mode.
    unsafe fn get_or_create_type() -> SCM;
}
pub use proc_macros::ForeignObject;
