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

//! Guile strings.

use {
    crate::{
        Guile,
        alloc::CAllocator,
        collections::{char_set::CharSet, list::List},
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        symbol::Symbol,
        sys::{
            scm_c_string_length, scm_char_set_to_string, scm_from_utf8_stringn, scm_is_string,
            scm_string, scm_string_equal_p, scm_string_null_p, scm_symbol_to_string,
            scm_to_utf8_stringn,
        },
        utils::{c_predicate, scm_predicate},
    },
    allocator_api2::vec::Vec,
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
    string::String as BufString,
};

/// Guile strings.
#[derive(Debug)]
#[repr(transparent)]
pub struct String<'gm> {
    scm: Scm<'gm>,
    _marker: PhantomData<&'gm ()>,
}
impl<'gm> String<'gm> {
    /// Create a string from utf8 strings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{string::String, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let string = String::from_str("", guile);
    ///     let string = String::from_str("hello world", guile);
    /// }).unwrap();
    /// ```
    pub fn from_str(string: &str, guile: &'gm Guile) -> Self {
        String {
            scm: Scm::from_ptr(
                unsafe { scm_from_utf8_stringn(string.as_bytes().as_ptr().cast(), string.len()) },
                guile,
            ),
            _marker: PhantomData,
        }
    }

    /// Return a newly allocated string from the contents of this string.
    ///
    /// # Exceptions
    ///
    /// There may be exceptions if the it fails to encode into utf8.
    pub fn as_string(&self) -> BufString<Vec<u8, CAllocator>> {
        let mut len = 0;
        let ptr = unsafe { scm_to_utf8_stringn(self.scm.as_ptr(), &raw mut len) }.cast::<u8>();

        // the documentation does not mention returning NULL.
        assert!(!ptr.is_null());

        // SAFETY: the string was allocated using `malloc`.
        let buffer = unsafe { Vec::from_raw_parts_in(ptr, len, len, CAllocator) };

        assert!(str::from_utf8(buffer.as_slice()).is_ok());
        // SAFETY: the returned string should be utf8, and we have an assertion above
        unsafe { BufString::from_utf8_unchecked(buffer) }
    }

    /// Get the length of a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{string::String, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(String::from_str("", guile).len(), 0);
    ///     assert_eq!(String::from_str("foo", guile).len(), 3);
    /// }).unwrap();
    /// ```
    pub fn len(&self) -> usize {
        unsafe { scm_c_string_length(self.scm.as_ptr()) }
    }
    /// Check if a string is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{string::String, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(String::from_str("", guile).is_empty());
    ///     assert!(!String::from_str("foo", guile).is_empty());
    /// }).unwrap();
    /// ```
    pub fn is_empty(&self) -> bool {
        scm_predicate(unsafe { scm_string_null_p(self.scm.as_ptr()) })
    }
}
impl<'gm> From<CharSet<'gm>> for String<'gm> {
    fn from(chrs: CharSet<'gm>) -> String<'gm> {
        String {
            scm: unsafe { Scm::from_ptr_unchecked(scm_char_set_to_string(chrs.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm> From<List<'gm, char>> for String<'gm> {
    fn from(list: List<'gm, char>) -> String<'gm> {
        String {
            scm: unsafe { Scm::from_ptr_unchecked(scm_string(list.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm> From<Symbol<'gm>> for String<'gm> {
    fn from(symbol: Symbol<'gm>) -> String<'gm> {
        String {
            scm: unsafe { Scm::from_ptr_unchecked(scm_symbol_to_string(symbol.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl PartialEq for String<'_> {
    fn eq(&self, r: &Self) -> bool {
        scm_predicate(unsafe { scm_string_equal_p(self.scm.as_ptr(), r.scm.as_ptr()) })
    }
}
unsafe impl ReprScm for String<'_> {}
impl<'gm> ToScm<'gm> for String<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm> TryFromScm<'gm> for String<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"string")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(unsafe { scm_is_string(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        String {
            scm,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile, std::ops::Deref};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_eq() {
        with_guile(|guile| {
            assert_eq!(
                String::from_str("hello", guile),
                String::from_str("hello", guile)
            );
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_len() {
        with_guile(|guile| {
            assert_eq!(String::from_str("world", guile).len(), 5,);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_is_empty() {
        with_guile(|guile| {
            assert_eq!(String::from_str("foo", guile).is_empty(), false,);
            assert_eq!(String::from_str("", guile).is_empty(), true,);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn into_string() {
        with_guile(|guile| {
            assert_eq!(
                String::from_str("210", guile),
                String::from(List::from_iter('0'..'3', guile))
            );
            assert_eq!(
                String::from_str("foo", guile),
                String::from(Symbol::from_str("foo", guile))
            );
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn to_string() {
        with_guile(|guile| {
            assert_eq!(String::from_str("asdf", guile).as_string().deref(), "asdf");
        })
        .unwrap();
    }
}
