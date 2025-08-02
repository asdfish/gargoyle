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

// TODO: reuse code from ../vector.rs since these are pretty similar

use {
    crate::{
        Guile,
        alloc::CAllocator,
        collections::list::List,
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        sys::{SCM, scm_array_handle_release, scm_t_array_handle},
        utils::scm_predicate,
    },
    allocator_api2::vec::Vec,
    std::{borrow::Cow, ffi::CStr, iter::FusedIterator, marker::PhantomData, num::NonZeroUsize},
};

pub(crate) trait ByteVectorType {
    const VECTOR_TYPE_NAME: &CStr;
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM;
}
impl ByteVectorType for u8 {
    const VECTOR_TYPE_NAME: &CStr = c"#u8()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_u8vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u8vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u8vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_u8vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_u8vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_u8vector;
}
impl ByteVectorType for u16 {
    const VECTOR_TYPE_NAME: &CStr = c"#u16()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_u16vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u16vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u16vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_u16vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_u16vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_u16vector;
}
impl ByteVectorType for u32 {
    const VECTOR_TYPE_NAME: &CStr = c"#u32()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_u32vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u32vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u32vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_u32vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_u32vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_u32vector;
}
impl ByteVectorType for u64 {
    const VECTOR_TYPE_NAME: &CStr = c"#u64()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_u64vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u64vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_u64vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_u64vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_u64vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_u64vector;
}
impl ByteVectorType for i8 {
    const VECTOR_TYPE_NAME: &CStr = c"#s8()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_s8vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s8vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s8vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_s8vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_s8vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_s8vector;
}
impl ByteVectorType for i16 {
    const VECTOR_TYPE_NAME: &CStr = c"#s16()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_s16vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s16vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s16vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_s16vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_s16vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_s16vector;
}
impl ByteVectorType for i32 {
    const VECTOR_TYPE_NAME: &CStr = c"#s32()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_s32vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s32vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s32vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_s32vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_s32vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_s32vector;
}
impl ByteVectorType for i64 {
    const VECTOR_TYPE_NAME: &CStr = c"#s64()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_s64vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s64vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_s64vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_s64vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_s64vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_s64vector;
}
impl ByteVectorType for f32 {
    const VECTOR_TYPE_NAME: &CStr = c"#f32()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_f32vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_f32vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_f32vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_f32vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_f32vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_f32vector;
}
impl ByteVectorType for f64 {
    const VECTOR_TYPE_NAME: &CStr = c"#f64()";
    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_list_to_f64vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_f64vector_to_list;

    const PREDICATE: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_f64vector_p;

    const ELEMENTS: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self = crate::sys::scm_f64vector_elements;
    const ELEMENTS_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self = crate::sys::scm_f64vector_writable_elements;

    const TAKE: unsafe extern "C" fn(_: *const Self, _: usize) -> SCM =
        crate::sys::scm_take_f64vector;
}

#[repr(transparent)]
pub struct ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    pub(crate) scm: Scm<'gm>,
    _marker: PhantomData<T>,
}
impl<'gm, T> ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    pub fn iter<'a>(&'a self) -> Iter<'a, 'gm, T> {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            T::ELEMENTS(
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
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, 'gm, T> {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            T::ELEMENTS_MUT(
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
impl<'gm, T> From<List<'gm, T>> for ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    fn from(list: List<'gm, T>) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr_unchecked(T::FROM_LIST(list.scm.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> From<Vec<T, CAllocator>> for ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    fn from(vec: Vec<T, CAllocator>) -> Self {
        let (ptr, len, _) = vec.into_raw_parts();

        Self {
            scm: unsafe { Scm::from_ptr_unchecked(T::TAKE(ptr, len)) },
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> IntoIterator for ByteVector<'gm, T>
where
    T: ByteVectorType + 'gm,
{
    type Item = T;
    type IntoIter = IntoIter<'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut handle = Default::default();
        let mut len = 0;
        let mut step = 0;
        let ptr = unsafe {
            T::ELEMENTS(
                self.scm.as_ptr(),
                &raw mut handle,
                &raw mut len,
                &raw mut step,
            )
        };

        IntoIter {
            handle,
            len: NonZeroUsize::new(len),
            step,
            ptr,
            _marker: PhantomData,
        }
    }
}
impl<'a, 'gm, T> IntoIterator for &'a ByteVector<'gm, T>
where
    T: ByteVectorType + 'gm,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, 'gm, T> IntoIterator for &'a mut ByteVector<'gm, T>
where
    T: ByteVectorType + 'gm,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
unsafe impl<T> ReprScm for ByteVector<'_, T> where T: ByteVectorType {}
impl<'gm, T> ToScm<'gm> for ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm, T> TryFromScm<'gm> for ByteVector<'gm, T>
where
    T: ByteVectorType,
{
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(T::VECTOR_TYPE_NAME)
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { T::PREDICATE(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

pub struct IntoIter<'gm, T>
where
    T: ByteVectorType,
{
    handle: scm_t_array_handle,
    ptr: *const T,
    len: Option<NonZeroUsize>,
    step: isize,
    _marker: PhantomData<&'gm T>,
}
impl<T> Drop for IntoIter<'_, T>
where
    T: ByteVectorType,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<'gm, T> DoubleEndedIterator for IntoIter<'gm, T>
where
    T: ByteVectorType,
{
    fn next_back(&mut self) -> Option<T> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(self.step * isize::try_from(len).unwrap()) };

                Some(unsafe { ptr.read() })
            }
            _ => None,
        }
    }
}
impl<'gm, T> ExactSizeIterator for IntoIter<'gm, T> where T: ByteVectorType {}
impl<'gm, T> FusedIterator for IntoIter<'gm, T> where T: ByteVectorType {}
impl<'gm, T> Iterator for IntoIter<'gm, T>
where
    T: ByteVectorType,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                Some(unsafe { ptr.read() })
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

pub struct Iter<'a, 'gm, T>
where
    T: ByteVectorType,
{
    handle: scm_t_array_handle,
    ptr: *const T,
    len: Option<NonZeroUsize>,
    step: isize,
    _marker: PhantomData<&'a &'gm T>,
}
impl<T> Drop for Iter<'_, '_, T>
where
    T: ByteVectorType,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<T> DoubleEndedIterator for Iter<'_, '_, T>
where
    T: ByteVectorType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(self.step * isize::try_from(len).unwrap()) };

                unsafe { ptr.as_ref() }
            }
            _ => None,
        }
    }
}
impl<T> ExactSizeIterator for Iter<'_, '_, T> where T: ByteVectorType {}
impl<T> FusedIterator for Iter<'_, '_, T> where T: ByteVectorType {}
impl<'a, 'gm, T> Iterator for Iter<'a, 'gm, T>
where
    T: ByteVectorType,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                unsafe { ptr.as_ref() }
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

pub struct IterMut<'a, 'gm, T>
where
    T: ByteVectorType,
{
    handle: scm_t_array_handle,
    ptr: *mut T,
    len: Option<NonZeroUsize>,
    step: isize,
    _marker: PhantomData<&'a &'gm T>,
}
impl<T> Drop for IterMut<'_, '_, T>
where
    T: ByteVectorType,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.handle);
        }
    }
}
impl<T> DoubleEndedIterator for IterMut<'_, '_, T>
where
    T: ByteVectorType,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                let len = len.get() - 1;
                self.len = NonZeroUsize::new(len);
                let ptr = unsafe { ptr.offset(self.step * isize::try_from(len).unwrap()) };

                unsafe { ptr.as_mut() }
            }
            _ => None,
        }
    }
}
impl<T> ExactSizeIterator for IterMut<'_, '_, T> where T: ByteVectorType {}
impl<T> FusedIterator for IterMut<'_, '_, T> where T: ByteVectorType {}
impl<'a, 'gm, T> Iterator for IterMut<'a, 'gm, T>
where
    T: ByteVectorType,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.ptr, self.len) {
            (ptr, Some(len)) if !ptr.is_null() => {
                self.ptr = unsafe { ptr.offset(self.step) };
                self.len = NonZeroUsize::new(len.get() - 1);

                unsafe { ptr.as_mut() }
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
    fn byte_vector_from() {
        with_guile(|guile| {
            let mut vector = ByteVector::from(List::from_iter([3, 2, 1], guile));
            vector.iter_mut().for_each(|i| *i += 1);
            assert_eq!(vector.iter().copied().collect::<Vec<_>>(), [2, 3, 4]);
            assert_eq!(vector.into_iter().rev().collect::<Vec<_>>(), [4, 3, 2]);
        })
        .unwrap();
    }
}
