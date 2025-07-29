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
        sys::{scm_char_set_eq, scm_char_set_hash, scm_char_set_leq, scm_char_set_p},
    },
    std::{
        borrow::Cow,
        cmp::Ordering,
        ffi::CStr,
        hash::{Hash, Hasher},
    },
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct CharSet<'id>(Scm<'id>);
impl Hash for CharSet<'_> {
    fn hash<H>(&self, h: &mut H)
    where
        H: Hasher,
    {
        let hash = unsafe {
            Scm::from_ptr(scm_char_set_hash(
                self.0.as_ptr(),
                usize::MAX.construct().as_ptr(),
            ))
        }
        .get::<usize>()
        .expect("failed to create hash");
        h.write_usize(hash);
    }
}
impl PartialEq for CharSet<'_> {
    fn eq(&self, r: &Self) -> bool {
        unsafe { Scm::from_ptr(scm_char_set_eq(self.0.as_ptr(), r.0.as_ptr())) }.is_true()
    }
}
impl PartialOrd for CharSet<'_> {
    fn partial_cmp(&self, r: &Self) -> Option<Ordering> {
        if self == r {
            Some(Ordering::Equal)
        } else if unsafe { Scm::from_ptr(scm_char_set_leq(self.0.as_ptr(), r.0.as_ptr())) }
            .is_true()
        {
            Some(Ordering::Less)
        } else if unsafe { Scm::from_ptr(scm_char_set_leq(r.0.as_ptr(), self.0.as_ptr())) }
            .is_true()
        {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}
impl<'id> ScmTy<'id> for CharSet<'id> {
    type Output = Self;

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"char-set")
    }

    fn construct(self) -> Scm<'id> {
        self.0
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_char_set_p(scm.as_ptr())).is_true() }
    }

    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self::Output {
        Self(scm)
    }
}

#[derive(Clone, Debug)]
#[expect(dead_code)]
pub struct CharSetIterator<'id>(CharSet<'id>, Scm<'id>);
// impl Iterator for CharSetIterator<>
