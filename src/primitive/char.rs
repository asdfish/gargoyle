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
        sys::{scm_char_p, scm_char_to_integer, scm_integer_to_char},
        utils::scm_predicate,
    },
    std::{borrow::Cow, ffi::CStr},
};

impl<'gm> ToScm<'gm> for char {
    fn to_scm(self, guile: &'gm Guile) -> Scm<'gm> {
        let scm = u32::from(self).to_scm(guile).as_ptr();
        Scm::from_ptr(unsafe { scm_integer_to_char(scm) }, guile)
    }
}
impl<'gm> TryFromScm<'gm> for char {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"char")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_char_p(scm.as_ptr()) })
    }
    unsafe fn from_scm_unchecked(scm: Scm<'gm>, guile: &'gm Guile) -> Self {
        u32::try_from_scm(
            Scm::from_ptr(unsafe { scm_char_to_integer(scm.as_ptr()) }, guile),
            guile,
        )
        .ok()
        .and_then(|ch| char::try_from(ch).ok())
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_conv() {
        with_guile(|guile| {
            (char::MIN..=char::MAX).for_each(|ch| {
                assert_eq!(char::try_from_scm(ch.to_scm(guile), guile), Ok(ch));
            });
        })
        .unwrap();
    }
}
