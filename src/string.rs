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
        alloc::CAllocator,
        sys::{scm_c_string_length, scm_is_string, scm_to_utf8_stringn},
    },
    allocator_api2::{alloc::AllocError, vec::Vec},
    std::{
        borrow::Cow,
        ffi::CStr,
        fmt::{self, Display, Formatter},
    },
    string::String as AString,
};

impl Api {
    pub fn make_string<'id, S>(&'id self, string: &S) -> String<'id>
    where
        S: AsRef<str> + ?Sized,
    {
        let string = string.as_ref();
        let scm =
            unsafe { crate::sys::scm_from_utf8_stringn(string.as_ptr().cast(), string.len()) };
        String(unsafe { Scm::from_ptr(scm) })
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct String<'id>(pub(crate) Scm<'id>);
impl String<'_> {
    pub fn len(&self) -> usize {
        unsafe { scm_c_string_length(self.0.as_ptr()) }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn to_string(&self) -> Result<AString<Vec<u8, CAllocator>>, AllocError> {
        let mut len: usize = 0;
        // SAFETY: since we have the lifetime, we can assume we're in guile mode
        let ptr = unsafe { scm_to_utf8_stringn(self.0.as_ptr(), &raw mut len) };
        if ptr.is_null() {
            Err(AllocError)
        } else {
            // SAFETY: we checked for null and since we don't know the capacity we must use length, and the pointer must be freed with [crate::sys::free]
            let vec = unsafe { Vec::from_raw_parts_in(ptr.cast(), len, len, CAllocator) };

            // this violates the contract so we should abort.
            assert!(
                str::from_utf8(&vec).is_ok(),
                "The returned string from `scm_to_utf8_stringn` was not utf8. This is a bug with guile."
            );

            // SAFETY: we have an assertion above
            Ok(unsafe { AString::from_utf8_unchecked(vec) })
        }
    }
}
impl Display for String<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        fn to_fmt_error<T>(_: T) -> fmt::Error {
            fmt::Error
        }

        self.to_string()
            .map_err(to_fmt_error)
            .and_then(|string| string.fmt(f).map_err(to_fmt_error))
    }
}
// SAFETY: This is `#[repr(transparent)]` and its only field is a [Scm].
unsafe impl<'id> ReprScm<'id> for String<'id> {}
impl<'id> ScmTy<'id> for String<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"string")
    }

    fn construct(self) -> Scm<'id> {
        self.0
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { scm_is_string(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, string: Scm<'id>) -> Self {
        Self(string)
    }
}
