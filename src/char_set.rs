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
        Api, Scm, ScmTy,
        list::List,
        string::String,
        sys::{
            SCM_UNDEFINED, scm_char_set_contains_p, scm_char_set_p, scm_list_to_char_set,
            scm_string_to_char_set,
        },
    },
    std::{borrow::Cow, ffi::CStr},
};

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct CharSet<'id>(Scm<'id>);
impl<'id> CharSet<'id> {
    pub fn contains(&self, ch: char) -> bool {
        unsafe {
            Scm::from_ptr(scm_char_set_contains_p(
                self.0.as_ptr(),
                char::construct(ch).as_ptr(),
            ))
        }
        .is_true()
    }
}
impl<'id> From<List<'id, char>> for CharSet<'id> {
    fn from(list: List<'id, char>) -> Self {
        Self(unsafe { Scm::from_ptr(scm_list_to_char_set(list.pair.as_ptr(), SCM_UNDEFINED)) })
    }
}
impl<'id> From<String<'id>> for CharSet<'id> {
    fn from(string: String<'id>) -> Self {
        Self(unsafe { Scm::from_ptr(scm_string_to_char_set(string.0.as_ptr(), SCM_UNDEFINED)) })
    }
}
impl<'id> ScmTy<'id> for CharSet<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"char-set")
    }

    fn construct(self) -> Scm<'id> {
        self.0
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_char_set_p(scm.as_ptr())).is_true() }
    }

    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self(scm)
    }
}

// #[derive(Clone, Debug)]
// #[repr(transparent)]
// pub struct Cursor<'id>(Scm<'id>);

// #[derive(Clone, Debug)]
// pub struct CharSetIterator<'id>(CharSet<'id>, Scm<'id>);

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_set_contains() {
        with_guile(|api| {
            let chars = CharSet::from(api.make_string("hi"));
            assert!(chars.contains('h'));
            assert!(chars.contains('i'));
            assert!(!chars.contains('o'));
        })
        .unwrap();
    }
}
