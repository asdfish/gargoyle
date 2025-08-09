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
