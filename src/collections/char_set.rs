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

//! Hash set of characters

use {
    crate::{
        Guile,
        collections::list::List,
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        string::String,
        sys::{
            SCM_UNDEFINED, scm_char_set_contains_p, scm_char_set_cursor, scm_char_set_cursor_next,
            scm_char_set_p, scm_char_set_ref, scm_end_of_char_set_p, scm_list_to_char_set,
            scm_string_to_char_set, scm_to_char_set,
        },
        utils::scm_predicate,
    },
    std::{borrow::Cow, ffi::CStr},
};

/// Character hash sets.
#[derive(Debug)]
#[repr(transparent)]
pub struct CharSet<'gm>(Scm<'gm>);
impl<'gm> CharSet<'gm> {
    /// Check if the character set contains a character.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, collections::char_set::CharSet, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let abc = CharSet::from(list!(guile, 'a', 'b', 'c'));
    ///     ('a'..='c')
    ///         .for_each(|ch| assert!(abc.contains(ch)));
    ///     ('d'..='z')
    ///         .for_each(|ch| assert!(!abc.contains(ch)));
    /// }).unwrap();
    /// ```
    pub fn contains(&self, ch: char) -> bool {
        let guile = unsafe { Guile::new_unchecked_ref() };
        scm_predicate(unsafe {
            scm_char_set_contains_p(self.0.as_ptr(), ch.to_scm(guile).as_ptr())
        })
    }

    /// Get an iterator over all characters.
    ///
    /// The order by which the characters appear is unspecified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, collections::char_set::CharSet, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut abc = CharSet::from(list!(guile, 'a', 'b', 'c')).iter().collect::<Vec<_>>();
    ///     abc.sort();
    ///     assert_eq!(abc, ['a', 'b', 'c']);
    /// }).unwrap();
    /// ```
    pub fn iter<'a>(&'a self) -> Iter<'a, 'gm> {
        Iter {
            cursor: unsafe { Scm::from_ptr_unchecked(scm_char_set_cursor(self.0.as_ptr())) },
            char_set: self,
        }
    }
}
impl<'gm> From<char> for CharSet<'gm> {
    fn from(ch: char) -> Self {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Self(Scm::from_ptr(
            unsafe { scm_to_char_set(ch.to_scm(guile).as_ptr()) },
            guile,
        ))
    }
}
impl<'gm> From<List<'gm, char>> for CharSet<'gm> {
    fn from(list: List<'gm, char>) -> Self {
        Self(unsafe { Scm::from_ptr_unchecked(scm_list_to_char_set(list.as_ptr(), SCM_UNDEFINED)) })
    }
}
impl<'gm> From<String<'gm>> for CharSet<'gm> {
    fn from(string: String<'gm>) -> Self {
        Self(unsafe {
            Scm::from_ptr_unchecked(scm_string_to_char_set(string.as_ptr(), SCM_UNDEFINED))
        })
    }
}
unsafe impl ReprScm for CharSet<'_> {}
impl<'gm> ToScm<'gm> for CharSet<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.0
    }
}
impl<'gm> TryFromScm<'gm> for CharSet<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"char-set")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_char_set_p(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self(scm)
    }
}

/// Iterator created by [CharSet::iter
pub struct Iter<'a, 'gm> {
    cursor: Scm<'gm>,
    char_set: &'a CharSet<'gm>,
}
impl Iterator for Iter<'_, '_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if scm_predicate(unsafe { scm_end_of_char_set_p(self.cursor.as_ptr()) }) {
            None
        } else {
            let guile = unsafe { Guile::new_unchecked_ref() };
            let ch = unsafe {
                char::from_scm_unchecked(
                    Scm::from_ptr_unchecked(scm_char_set_ref(
                        self.char_set.0.as_ptr(),
                        self.cursor.as_ptr(),
                    )),
                    guile,
                )
            };
            unsafe {
                scm_char_set_cursor_next(self.char_set.0.as_ptr(), self.cursor.as_ptr());
            }

            Some(ch)
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile, std::collections::HashSet};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_set_iter() {
        with_guile(|guile| {
            let set = CharSet::from(String::from_str("asdf", guile));
            assert_eq!(
                set.iter().collect::<HashSet<char>>(),
                HashSet::from_iter("asdf".chars())
            );
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_set_contains() {
        with_guile(|guile| {
            let set = CharSet::from(List::from_iter(
                "thequickbrownfoxjumpsoverthelazydog".chars(),
                guile,
            ));
            ('a'..='z').for_each(|ch| assert!(set.contains(ch)));
        })
        .unwrap();
    }
}
