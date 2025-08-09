// garguile - guile bindings for rust
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
        sys::{SCM_BOOL_F, SCM_BOOL_T, scm_is_bool},
        utils::c_predicate,
    },
    std::{borrow::Cow, ffi::CStr},
};

impl<'gm> TryFromScm<'gm> for bool {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"bool")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(unsafe { scm_is_bool(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        scm.is_true()
    }
}
impl<'gm> ToScm<'gm> for bool {
    fn to_scm(self, guile: &'gm Guile) -> Scm<'gm> {
        Scm::from_ptr(
            match self {
                true => unsafe { SCM_BOOL_T },
                false => unsafe { SCM_BOOL_F },
            },
            guile,
        )
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn bool_conv() {
        with_guile(|guile| {
            [true, false]
                .into_iter()
                .for_each(|b| assert_eq!(bool::try_from_scm(b.to_scm(&guile), &guile), Ok(b)));
        })
        .unwrap();
    }
}
