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
    crate::sys::{SCM, scm_is_true},
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, c_int},
        fmt::{self, Display, Formatter},
    },
};

pub fn c_predicate<F>(f: F) -> bool
where
    F: FnOnce() -> c_int,
{
    f() != 0
}

pub fn scm_predicate<F>(f: F) -> bool
where
    F: FnOnce() -> SCM,
{
    c_predicate(|| unsafe { scm_is_true(f()) })
}

pub trait CowCStrExt<'a> {
    fn display(&'a self) -> CowCStrDisplay<'a>;
}
impl<'a> CowCStrExt<'a> for Cow<'a, CStr> {
    fn display(&'a self) -> CowCStrDisplay<'a> {
        CowCStrDisplay(self)
    }
}
pub struct CowCStrDisplay<'a>(&'a Cow<'a, CStr>);
impl<'a> Display for CowCStrDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        BStr::new(self.0.as_ref().to_bytes()).fmt(f)
    }
}
