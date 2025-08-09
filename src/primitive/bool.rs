// garguile - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
