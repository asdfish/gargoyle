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
    crate::{Api, Scm, ScmTy, sys::scm_char_set_p},
    std::ffi::CStr,
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct CharSet<'id>(Scm<'id>);
impl<'id> ScmTy<'id> for CharSet<'id> {
    type Output = Self;

    const TYPE_NAME: &'static CStr = c"char set";

    fn construct(self) -> Scm<'id> {
        self.0
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_char_set_p(scm.as_ptr())).is_true() }
    }

    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        Self(unsafe { (*scm).cast_lifetime() })
    }
}
