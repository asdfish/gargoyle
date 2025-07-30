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
        Api, ReprScm, Scm, ScmTy,
        list::List,
        sys::{
            scm_array_handle_release, scm_is_vector, scm_t_array_handle, scm_vector,
            scm_vector_writable_elements,
        },
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator},
        marker::PhantomData,
        num::{NonZeroIsize, NonZeroUsize},
        ptr::{self, NonNull},
    },
};

#[derive(Debug)]
#[repr(transparent)]
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
    pub fn iter<'borrow>(&'borrow self) -> Iter<'borrow, 'id, T>
    where
        T: ReprScm<'id>,
    {
        Iter::new(&self.scm)
    }
    pub fn iter_mut<'borrow>(&'borrow mut self) -> IterMut<'borrow, 'id, T>
    where
        T: ReprScm<'id>,
    {
        IterMut::new(&mut self.scm)
    }
}
impl<'id, T> From<List<'id, T>> for Vector<'id, T>
where
    T: ScmTy<'id>,
{
    fn from(list: List<'id, T>) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr(scm_vector(list.pair.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
// SAFETY: This is `#[repr(transparent)]` and its only field is a [Scm].
unsafe impl<'id, T> ReprScm<'id> for Vector<'id, T> where T: ScmTy<'id> {}
impl<'id, T> ScmTy<'id> for Vector<'id, T>
where
    T: ScmTy<'id>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "#{}()",
            BStr::new(T::type_name().as_ref().to_bytes())
        ))
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(c"#()"))
    }
    fn construct(self) -> Scm<'id> {
        self.scm
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { scm_is_vector(scm.as_ptr()) }
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
    T: ReprScm<'id>,
{
    handle: scm_t_array_handle,
    len: Option<NonZeroUsize>,
    step: Option<NonZeroIsize>,
    ptr: Option<&'borrow T>,
    _marker: PhantomData<&'id T>,
}
impl<'borrow, 'id, T> Iter<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    fn new(array: &'borrow Scm<'id>) -> Self {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            scm_vector_writable_elements(
                array.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        }
        .cast::<T>();

        Iter {
            handle,
            len: NonZeroUsize::new(len),
            step: NonZeroIsize::new(step),
            ptr: unsafe { ptr.as_ref() },
            _marker: PhantomData,
        }
    }
}
impl<'borrow, 'id, T> DoubleEndedIterator for Iter<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), Some(ptr)) => {
                let ptr = unsafe {
                    ptr::from_ref(ptr).offset(isize::try_from(len.get() - 1).unwrap() * step.get())
                };
                self.len = NonZeroUsize::new(len.get() - 1);
                unsafe { ptr.as_ref() }
            }
            _ => None,
        }
    }
}
impl<'id, T> Drop for Iter<'_, 'id, T>
where
    T: ReprScm<'id>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'id, T> ExactSizeIterator for Iter<'_, 'id, T> where T: ReprScm<'id> {}
impl<'id, T> FusedIterator for Iter<'_, 'id, T> where T: ReprScm<'id> {}
impl<'borrow, 'id, T> Iterator for Iter<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    type Item = &'borrow T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), Some(ptr)) => {
                self.ptr = unsafe { ptr::from_ref(ptr).offset(step.get()).as_ref() };
                self.len = NonZeroUsize::new(len.get() - 1);

                Some(ptr)
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

pub struct IterMut<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    handle: scm_t_array_handle,
    len: Option<NonZeroUsize>,
    step: Option<NonZeroIsize>,
    ptr: Option<NonNull<T>>,
    _marker: PhantomData<&'borrow &'id T>,
}
impl<'borrow, 'id, T> IterMut<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    fn new(array: &'borrow mut Scm<'id>) -> Self {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            scm_vector_writable_elements(
                array.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        }
        .cast::<T>();

        IterMut {
            handle,
            len: NonZeroUsize::new(len),
            step: NonZeroIsize::new(step),
            ptr: NonNull::new(ptr),
            _marker: PhantomData,
        }
    }
}
impl<'borrow, 'id, T> DoubleEndedIterator for IterMut<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), Some(ptr)) => {
                let ptr = unsafe {
                    ptr.as_ptr()
                        .offset(isize::try_from(len.get() - 1).unwrap() * step.get())
                };
                self.len = NonZeroUsize::new(len.get() - 1);
                unsafe { ptr.as_mut() }
            }
            _ => None,
        }
    }
}
impl<'id, T> Drop for IterMut<'_, 'id, T>
where
    T: ReprScm<'id>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'id, T> ExactSizeIterator for IterMut<'_, 'id, T> where T: ReprScm<'id> {}
impl<'id, T> FusedIterator for IterMut<'_, 'id, T> where T: ReprScm<'id> {}
impl<'borrow, 'id, T> Iterator for IterMut<'borrow, 'id, T>
where
    T: ReprScm<'id>,
{
    type Item = &'borrow mut T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), Some(mut ptr)) => {
                self.ptr = NonNull::new(unsafe { ptr.as_ptr().offset(step.get()) });
                self.len = NonZeroUsize::new(len.get() - 1);

                Some(unsafe { ptr.as_mut() })
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn vector_iter_mut() {
        with_guile(|api| {
            let mut vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter_mut();
            assert_eq!(iter.next(), Some(&mut api.make_list([3])));
            assert_eq!(iter.next(), Some(&mut api.make_list([2])));
            assert_eq!(iter.next(), Some(&mut api.make_list([1])));
            assert_eq!(iter.next(), None);

            let mut vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter_mut();
            assert_eq!(iter.next_back(), Some(&mut api.make_list([1])));
            assert_eq!(iter.next_back(), Some(&mut api.make_list([2])));
            assert_eq!(iter.next_back(), Some(&mut api.make_list([3])));
            assert_eq!(iter.next_back(), None);

            let mut vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter_mut();
            assert_eq!(iter.next(), Some(&mut api.make_list([3])));
            assert_eq!(iter.next_back(), Some(&mut api.make_list([1])));
            assert_eq!(iter.next(), Some(&mut api.make_list([2])));
            assert_eq!(iter.next_back(), None);

            let vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter();
            assert_eq!(iter.next(), Some(&api.make_list([3])));

            let mut iter2 = vec.iter();
            assert_eq!(iter2.next(), Some(&api.make_list([3])));

            assert_eq!(iter.next(), Some(&api.make_list([2])));
            assert_eq!(iter.next(), Some(&api.make_list([1])));
            assert_eq!(iter.next(), None);

            let vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter();
            assert_eq!(iter.next_back(), Some(&api.make_list([1])));
            assert_eq!(iter.next_back(), Some(&api.make_list([2])));
            assert_eq!(iter.next_back(), Some(&api.make_list([3])));
            assert_eq!(iter.next_back(), None);

            let vec = Vector::from(api.make_list([
                api.make_list([1]),
                api.make_list([2]),
                api.make_list([3]),
            ]));
            let mut iter = vec.iter();
            assert_eq!(iter.next(), Some(&api.make_list([3])));
            assert_eq!(iter.next_back(), Some(&api.make_list([1])));
            assert_eq!(iter.next(), Some(&api.make_list([2])));
            assert_eq!(iter.next_back(), None);
        })
        .unwrap();
    }
}
