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

use {crate::sys::SCM, std::marker::PhantomData};

pub struct Scm<'guile_mode> {
    ptr: SCM,
    _marker: PhantomData<&'guile_mode ()>,
}
impl Scm<'_> {
    pub fn as_ptr(&self) -> SCM {
        self.ptr
    }
    /// # Safety
    ///
    /// You must ensure that the lifetime is attached to a [Guile][crate::Guile] object to ensure that it is in guile mode.
    pub unsafe fn from_ptr(ptr: SCM) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
}

/// Trait for types that can be converted from [Scm].
///
/// # Safety
///
/// To implement this trait safely, you must ensure that [Self::from_scm] is safe to evaluate if [Self::predicate] returns [true].
pub unsafe trait FromScm<'gm> {
    fn predicate(_: &Scm<'gm>) -> bool;
    unsafe fn from_scm_unchecked(_: Scm<'gm>) -> Self;
}
pub trait ToScm<'guile_mode> {
    fn to_scm(self) -> Scm<'guile_mode>
    where
        Self: Sized;
}

unsafe impl<'gm> FromScm<'gm> for Scm<'gm> {
    fn predicate(_: &Scm<'gm>) -> bool {
        true
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>) -> Self {
        scm
    }
}
impl<'gm> ToScm<'gm> for Scm<'gm> {
    fn to_scm(self) -> Scm<'gm> {
        self
    }
}
