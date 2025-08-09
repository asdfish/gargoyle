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

//! Memory backed vectors.

use {
    crate::{
        Guile,
        collections::list::List,
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        sys::{
            SCM, scm_array_handle_release, scm_c_make_vector, scm_t_array_handle, scm_vector,
            scm_vector_elements, scm_vector_p, scm_vector_writable_elements,
        },
        utils::{CowCStrExt, scm_predicate},
    },
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        iter::FusedIterator,
        marker::PhantomData,
        num::NonZeroUsize,
    },
};

/// Vector backed by a contiguous block of memory.
#[repr(transparent)]
pub struct Vector<'gm, T> {
    scm: Scm<'gm>,
    _marker: PhantomData<T>,
}
impl<'gm, T> From<List<'gm, T>> for Vector<'gm, T> {
    fn from(list: List<'gm, T>) -> Self {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Self {
            scm: Scm::from_ptr(unsafe { scm_vector(list.as_ptr()) }, guile),
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> Vector<'gm, T> {
    /// Create a vector of copied items.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::vector::Vector, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(
    ///         Vector::new(10, 10, guile).into_iter().collect::<Vec<_>>(),
    ///         [10; 10],
    ///     );
    /// }).unwrap();
    /// ```
    pub fn new(of: T, n: usize, guile: &'gm Guile) -> Self
    where
        T: Copy + ToScm<'gm>,
    {
        Self {
            scm: unsafe {
                Scm::from_ptr_unchecked(scm_c_make_vector(n, of.to_scm(guile).as_ptr()))
            },
            _marker: PhantomData,
        }
    }

    /// Get an immutable iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::vector::Vector, reference::Ref, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     Vector::new(true, 10, guile)
    ///         .iter()
    ///         .map(Ref::copied)
    ///         .for_each(|i| assert!(i));
    /// }).unwrap();
    /// ```
    pub fn iter<'a>(&'a self) -> Iter<'a, 'gm, T>
    where
        T: TryFromScm<'gm>,
    {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            scm_vector_elements(
                self.scm.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        };

        Iter {
            handle,
            ptr,
            len: NonZeroUsize::new(len),
            step,
            _marker: PhantomData,
        }
    }

    /// Get a mutable iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::{list::List, pair::Pair, vector::Vector}, reference::Ref, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut vec = Vector::from(List::from_iter([(); 10].map(|_| Pair::new(false, (), guile)), guile));
    ///     vec
    ///         .iter_mut()
    ///         .for_each(|mut pair| pair.set_car(true));
    ///     vec
    ///         .into_iter()
    ///         .for_each(|i| assert!(i.as_car().copied()));
    /// }).unwrap();
    /// ```
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, 'gm, T>
    where
        T: TryFromScm<'gm>,
    {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            scm_vector_writable_elements(
                self.scm.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        };

        IterMut {
            handle,
            ptr,
            len: NonZeroUsize::new(len),
            step,
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> IntoIterator for Vector<'gm, T>
where
    T: TryFromScm<'gm> + 'gm,
{
    type Item = T;
    type IntoIter = IntoIter<'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            scm_vector_elements(
                self.scm.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        };

        IntoIter {
            handle,
            ptr,
            len: NonZeroUsize::new(len),
            step,
            _marker: PhantomData,
        }
    }
}
impl<'a, 'gm, T> IntoIterator for &'a Vector<'gm, T>
where
    T: TryFromScm<'gm> + 'gm,
{
    type Item = Ref<'a, 'gm, T>;
    type IntoIter = Iter<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, 'gm, T> IntoIterator for &'a mut Vector<'gm, T>
where
    T: TryFromScm<'gm> + 'gm,
{
    type Item = RefMut<'a, 'gm, T>;
    type IntoIter = IterMut<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
unsafe impl<'gm, T> ReprScm for Vector<'gm, T> {}
impl<'gm, T> ToScm<'gm> for Vector<'gm, T> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm, T> TryFromScm<'gm> for Vector<'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!("(vector {})", T::type_name().display()))
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(c"vector"))
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_vector_p(scm.as_ptr()) }) && { todo!("type check all values") }
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

/// Iterator for [Vector::into_iter].
pub struct IntoIter<'gm, T>
where
    T: TryFromScm<'gm>,
{
    handle: scm_t_array_handle,
    ptr: *const SCM,
    step: isize,
    len: Option<NonZeroUsize>,
    _marker: PhantomData<&'gm T>,
}
impl<'gm, T> DoubleEndedIterator for IntoIter<'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(isize::try_from(len).unwrap() * self.step) };
                let guile = unsafe { Guile::new_unchecked_ref() };
                Some(unsafe { T::from_scm_unchecked(Scm::from_ptr(ptr.read(), guile), guile) })
            }
            _ => None,
        }
    }
}
impl<'gm, T> Drop for IntoIter<'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'gm, T> ExactSizeIterator for IntoIter<'gm, T> where T: TryFromScm<'gm> {}
impl<'gm, T> FusedIterator for IntoIter<'gm, T> where T: TryFromScm<'gm> {}
impl<'gm, T> Iterator for IntoIter<'gm, T>
where
    T: TryFromScm<'gm>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { self.ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                let guile = unsafe { Guile::new_unchecked_ref() };
                Some(unsafe { T::from_scm_unchecked(Scm::from_ptr(ptr.read(), guile), guile) })
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

/// Iterator for [Vector::iter].
pub struct Iter<'a, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    handle: scm_t_array_handle,
    ptr: *const SCM,
    step: isize,
    len: Option<NonZeroUsize>,
    _marker: PhantomData<&'a &'gm T>,
}
impl<'gm, T> DoubleEndedIterator for Iter<'_, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(isize::try_from(len).unwrap() * self.step) };
                Some(unsafe { Ref::new_unchecked(ptr.read()) })
            }
            _ => None,
        }
    }
}
impl<'gm, T> Drop for Iter<'_, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'gm, T> ExactSizeIterator for Iter<'_, 'gm, T> where T: TryFromScm<'gm> {}
impl<'gm, T> FusedIterator for Iter<'_, 'gm, T> where T: TryFromScm<'gm> {}
impl<'a, 'gm, T> Iterator for Iter<'a, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    type Item = Ref<'a, 'gm, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { self.ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                Some(unsafe { Ref::new_unchecked(ptr.read()) })
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

/// Iterator for [Vector::iter_mut].
pub struct IterMut<'a, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    handle: scm_t_array_handle,
    ptr: *mut SCM,
    step: isize,
    len: Option<NonZeroUsize>,
    _marker: PhantomData<&'a &'gm T>,
}
impl<'gm, T> DoubleEndedIterator for IterMut<'_, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(isize::try_from(len).unwrap() * self.step) };
                Some(unsafe { RefMut::new_unchecked(ptr.read()) })
            }
            _ => None,
        }
    }
}
impl<'gm, T> Drop for IterMut<'_, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'gm, T> ExactSizeIterator for IterMut<'_, 'gm, T> where T: TryFromScm<'gm> {}
impl<'gm, T> FusedIterator for IterMut<'_, 'gm, T> where T: TryFromScm<'gm> {}
impl<'a, 'gm, T> Iterator for IterMut<'a, 'gm, T>
where
    T: TryFromScm<'gm>,
{
    type Item = RefMut<'a, 'gm, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { self.ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                Some(unsafe { RefMut::new_unchecked(ptr.read()) })
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
    fn vector_construction() {
        with_guile(|guile| {
            assert_eq!(
                Vector::new('a', 3, guile).into_iter().collect::<String>(),
                "aaa"
            );
            assert_eq!(
                Vector::from(List::from_iter([3, 2, 1], guile))
                    .into_iter()
                    .sum::<usize>(),
                6
            );
        })
        .unwrap();
    }

    /// Test that we can have multiple handles
    #[cfg_attr(miri, ignore)]
    #[test]
    fn vector_iter() {
        with_guile(|guile| {
            let mut vec = Vector::from(List::from_iter([3, 2, 1], guile));
            {
                let _iter = vec.iter();
                let _iter = vec.iter();
                assert_eq!(
                    vec.iter()
                        .map(Ref::copied)
                        .zip(vec.iter().map(Ref::copied))
                        .collect::<Vec<_>>(),
                    [(1, 1), (2, 2), (3, 3)]
                );
            }
            assert_eq!(
                vec.iter_mut().map(RefMut::copied).collect::<Vec<_>>(),
                [1, 2, 3]
            );

            assert_eq!(vec.into_iter().rev().collect::<Vec<_>>(), [3, 2, 1]);
        })
        .unwrap();
    }
}
