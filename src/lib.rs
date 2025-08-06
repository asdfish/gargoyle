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

//! Rust bindings to guile.

#![expect(private_bounds)]

pub mod alloc;
pub mod collections;
pub mod dynwind;
mod error;
pub mod foreign_object;
mod guile_mode;
pub mod num;
mod primitive;
pub mod rand;
#[doc(hidden)]
pub mod reexports;
pub mod reference;
pub mod scm;
pub mod string;
pub mod subr;
pub mod symbol;
pub mod sys;
mod tuple;
mod utils;

use std::ptr::NonNull;

pub use guile_mode::*;

#[repr(transparent)]
pub struct Guile {
    _marker: (),
}
impl Guile {
    /// # Safety
    ///
    /// This can be run safely if you run it in guile mode and drop it before guile mode ends.
    pub unsafe fn new_unchecked() -> Self {
        Self { _marker: () }
    }

    /// # Safety
    ///
    /// You must be in guile mode or never dereference the returned reference.
    pub unsafe fn new_unchecked_ref<'a>() -> &'a Self {
        unsafe { NonNull::<Self>::dangling().as_ref() }
    }
    /// # Safety
    ///
    /// You must be in guile mode or never dereference the returned reference.
    pub unsafe fn new_unchecked_mut<'a>() -> &'a mut Self {
        unsafe { NonNull::<Self>::dangling().as_mut() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_refs() {
        unsafe {
            Guile::new_unchecked_ref();
        }
        unsafe {
            Guile::new_unchecked_mut();
        }
    }
}
