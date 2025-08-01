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
        sys::{SCM, scm_equal_p, scm_is_true},
        utils::c_predicate,
    },
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
};

#[derive(Debug)]
pub struct Scm<'guile_mode> {
    pub(crate) ptr: SCM,
    _marker: PhantomData<&'guile_mode ()>,
}
impl<'gm> Scm<'gm> {
    pub fn as_ptr(&self) -> SCM {
        self.ptr
    }
    pub fn from_ptr(ptr: SCM, _: &'gm Guile) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
    /// # Safety
    ///
    /// The lifetime of the [Scm] object should be tied to a [Guile] so that it will always be in guile mode.
    pub unsafe fn from_ptr_unchecked(ptr: SCM) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Compare equality using `equal?`
    pub fn is_equal(&self, r: &Self) -> bool {
        unsafe { Scm::from_ptr_unchecked(scm_equal_p(self.as_ptr(), r.as_ptr())) }.is_true()
    }

    pub fn is_true(&self) -> bool {
        c_predicate(|| unsafe { scm_is_true(self.as_ptr()) })
    }
}
impl PartialEq for Scm<'_> {
    /// See [Self::is_equal].
    fn eq(&self, r: &Self) -> bool {
        self.is_equal(r)
    }
}

pub trait TryFromScm<'guile_mode> {
    fn type_name() -> Cow<'static, CStr>;

    fn predicate(_: &Scm<'guile_mode>, _: &'guile_mode Guile) -> bool;

    fn try_from_scm(
        scm: Scm<'guile_mode>,
        guile: &'guile_mode Guile,
    ) -> Result<Self, Scm<'guile_mode>>
    where
        Self: Sized,
    {
        if Self::predicate(&scm, guile) {
            Ok(unsafe { Self::from_scm_unchecked(scm, guile) })
        } else {
            Err(scm)
        }
    }

    /// Create [Self] without type checking.
    ///
    /// # Safety
    ///
    /// [Self::predicate] should implement type checking.
    unsafe fn from_scm_unchecked(_: Scm<'guile_mode>, _: &'guile_mode Guile) -> Self
    where
        Self: Sized;
}
pub trait ToScm<'guile_mode> {
    fn to_scm(self, _: &'guile_mode Guile) -> Scm<'guile_mode>
    where
        Self: Sized;
}

impl<'gm> TryFromScm<'gm> for Scm<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"any")
    }

    fn predicate(_: &Scm<'gm>, _: &'gm Guile) -> bool {
        true
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        scm
    }
}
impl<'gm> ToScm<'gm> for Scm<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self
    }
}
