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
        sys::{
            SCM, scm_array_handle_release, scm_t_array_handle, scm_vector_elements, scm_vector_p,
        },
        with_guile_protected,
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
        num::{NonZeroIsize, NonZeroUsize},
        pin::Pin,
    },
};

pub struct Vector<'id, T>
where
    T: ScmTy<'id>,
{
    scm: Scm<'id>,
    _marker: PhantomData<T>,
}
impl<'id, T> Vector<'id, T>
where
    T: ScmTy<'id>,
{
    pub fn iter<'borrow>(&'borrow self) -> Iter<'borrow, 'id, T> {
        unsafe { Iter::new_unchecked(&self.scm) }
    }
}
impl<'id, T> ScmTy<'id> for Vector<'id, T>
where
    T: ScmTy<'id>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "#({})",
            BStr::new(T::type_name().as_ref().to_bytes())
        ))
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed(c"#()"))
    }
    fn construct(self) -> Scm<'id> {
        self.scm
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_vector_p(scm.as_ptr())) }.is_true() && {
            with_guile_protected(|_, g| {
                let mut iter = unsafe { Iter::new_unchecked(scm) };
                let iter = Pin::new(&mut iter);
                g.protect(iter).all(|i| {
                    let api = unsafe { Api::new_unchecked() };
                    T::predicate(&api, &i)
                })
            })
            .unwrap_or_default()
        }
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

pub struct Iter<'borrow, 'id, T>
where
    T: ScmTy<'id>,
{
    lock: scm_t_array_handle,
    ptr: *const SCM,
    len: Option<NonZeroUsize>,
    step: Option<NonZeroIsize>,
    _marker: PhantomData<&'borrow &'id T>,
}
impl<'borrow, 'id, T> Iter<'borrow, 'id, T>
where
    T: ScmTy<'id>,
{
    /// # Safety
    ///
    /// The scm must be a vector that only contains `T`.
    unsafe fn new_unchecked(vec: &'borrow Scm) -> Self {
        let mut lock = Default::default();
        let mut len = 0;
        let mut step = 0;

        Self {
            ptr: unsafe {
                scm_vector_elements(vec.as_ptr(), &raw mut lock, &raw mut len, &raw mut step)
            },
            lock,
            len: NonZeroUsize::new(len),
            step: NonZeroIsize::new(step),
            _marker: PhantomData,
        }
    }
}
impl<'id, T> DoubleEndedIterator for Iter<'_, 'id, T>
where
    T: ScmTy<'id>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len, self.step) {
            (ptr, Some(len), Some(step)) if !ptr.is_null() => {
                let api = unsafe { Api::new_unchecked() };
                let len = len.get() - 1;
                let ptr = unsafe { ptr.offset(step.get() * isize::try_from(len).unwrap()) };
                self.len = NonZeroUsize::new(len);

                if ptr.is_null() {
                    None
                } else {
                    Some(unsafe { T::get_unchecked(&api, Scm::from_ptr(ptr.read())) })
                }
            }
            _ => None,
        }
    }
}
impl<'id, T> Drop for Iter<'_, 'id, T>
where
    T: ScmTy<'id>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.lock);
        }
    }
}
impl<'id, T> Iterator for Iter<'_, 'id, T>
where
    T: ScmTy<'id>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len, self.step) {
            (ptr, Some(len), Some(step)) if !ptr.is_null() => {
                let api = unsafe { Api::new_unchecked() };
                self.len = NonZeroUsize::new(len.get() - 1);
                self.ptr = unsafe { ptr.offset(step.get()) };

                Some(unsafe { T::get_unchecked(&api, Scm::from_ptr(ptr.read())) })
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}
