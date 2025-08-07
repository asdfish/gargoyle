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
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        string::String,
        sys::{
            SCM, scm_c_symbol_length, scm_from_utf8_symbol, scm_from_utf8_symboln, scm_make_symbol,
            scm_string_to_symbol, scm_symbol_interned_p, scm_symbol_p,
        },
        utils::scm_predicate,
    },
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Symbol<'gm> {
    pub(crate) ptr: SCM,
    _marker: PhantomData<&'gm ()>,
}
impl<'gm> From<String<'gm>> for Symbol<'gm> {
    fn from(string: String<'gm>) -> Self {
        Self {
            ptr: unsafe { scm_string_to_symbol(string.scm.as_ptr()) },
            _marker: PhantomData,
        }
    }
}
impl<'gm> Symbol<'gm> {
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{with_guile, symbol::Symbol};
    /// with_guile(|guile| {
    ///     assert_eq!(Symbol::from_str("foo", guile).len(), 3);
    ///     assert_eq!(Symbol::from_str("", guile).len(), 0);
    /// }).unwrap();
    /// ```
    /// ```
    pub fn from_str(symbol: &str, _: &'gm Guile) -> Self {
        if symbol.is_empty() {
            // segfault with length 0
            Self {
                ptr: unsafe { scm_from_utf8_symbol(c"".as_ptr().cast()) },
                _marker: PhantomData,
            }
        } else {
            Self {
                // SAFETY: `str` is always utf8 and the second argument guarantees we are in guile mode.
                ptr: unsafe {
                    scm_from_utf8_symboln(symbol.as_bytes().as_ptr().cast(), symbol.len())
                },
                _marker: PhantomData,
            }
        }
    }

    pub fn len(&self) -> usize {
        unsafe { scm_c_symbol_length(self.ptr) }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn new_interned(string: &String<'gm>) -> Self {
        Self {
            ptr: unsafe { scm_make_symbol(string.scm.as_ptr()) },
            _marker: PhantomData,
        }
    }

    pub fn is_interned(&self) -> bool {
        scm_predicate(unsafe { scm_symbol_interned_p(self.ptr) })
    }
}
unsafe impl ReprScm for Symbol<'_> {}
impl<'gm> ToScm<'gm> for Symbol<'gm> {
    fn to_scm(self, guile: &'gm Guile) -> Scm<'gm> {
        Scm::from_ptr(self.ptr, guile)
    }
}
impl<'gm> TryFromScm<'gm> for Symbol<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"symbol")
    }
    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_symbol_p(scm.as_ptr()) })
    }
    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            ptr: scm.as_ptr(),
            _marker: PhantomData,
        }
    }
}
