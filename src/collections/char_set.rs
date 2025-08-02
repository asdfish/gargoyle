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

#[derive(Debug)]
#[repr(transparent)]
pub struct CharSet<'gm>(pub(crate) Scm<'gm>);
impl<'gm> CharSet<'gm> {
    pub fn contains(&self, ch: char) -> bool {
        let guile = unsafe { Guile::new_unchecked_ref() };
        scm_predicate(unsafe {
            scm_char_set_contains_p(self.0.as_ptr(), ch.to_scm(guile).as_ptr())
        })
    }

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
        Self(unsafe {
            Scm::from_ptr_unchecked(scm_list_to_char_set(list.scm.as_ptr(), SCM_UNDEFINED))
        })
    }
}
impl<'gm> From<String<'gm>> for CharSet<'gm> {
    fn from(string: String<'gm>) -> Self {
        Self(unsafe {
            Scm::from_ptr_unchecked(scm_string_to_char_set(string.scm.as_ptr(), SCM_UNDEFINED))
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
        });
    }
}
