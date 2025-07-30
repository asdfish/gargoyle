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
        Api, ReprScm, Scm, ScmTy,
        num::{NumTy, ScmNum},
        sys::{SCM, scm_imag_part, scm_is_complex, scm_real_part},
    },
    std::{borrow::Cow, ffi::CStr},
};

#[derive(Debug)]
#[repr(transparent)]
pub struct Complex<'id>(Scm<'id>);
impl<'id> Complex<'id> {
    /// Get the real part.
    pub fn real(&self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_real_part(self.0.as_ptr())) }
    }
    /// Get the real part.
    pub fn imag(&self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_imag_part(self.0.as_ptr())) }
    }
}
impl<'id> NumTy<'id> for Complex<'id> {}
// SAFETY: This is `#[repr(transparent)]` and its only field is a [Scm].
unsafe impl<'id> ReprScm<'id> for Complex<'id> {}
impl<'id> ScmNum<'id> for Complex<'id> {
    unsafe fn as_ptr(&self) -> SCM {
        unsafe { self.0.as_ptr() }
    }
    fn is_real(&self) -> bool {
        false
    }
}
impl<'id> ScmTy<'id> for Complex<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"complex")
    }

    fn construct(self) -> Scm<'id> {
        unsafe { self.0.cast_lifetime() }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { scm_is_complex(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self(scm)
    }
}
