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

// TODO: these implementations are probably unsafe

use {
    crate::{
        Api, Scm, ScmTy,
        list::List,
        sys::{SCM, scm_array_handle_release, scm_t_array_handle},
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        iter::FusedIterator,
        marker::PhantomData,
        num::{NonZeroIsize, NonZeroUsize},
    },
};

/// Marker traits for types that may be safely transmuted
pub unsafe trait SafeTransmute<T>
where
    Self: Sized,
    T: Sized,
{
}
// SAFETY: all types should be transmutable to themself
unsafe impl<T> SafeTransmute<T> for T {}
// SAFETY: these types are `#[repr(transparent)]` onto [Scm].
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::sys::SCM {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::char_set::CharSet<'id> {}
unsafe impl<'id, T> SafeTransmute<Scm<'id>> for crate::list::List<'id, T> where T: ScmTy<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::complex::Complex<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::rational::Rational<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::ExactInteger<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::Number<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::string::String<'id> {}

#[derive(Debug)]
#[repr(transparent)]
pub struct Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    scm: Scm<'id>,
    _marker: PhantomData<&'id D>,
}
impl<'id, D> Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn make_iter<'borrow, I>(&'borrow self) -> VectorIterator<'borrow, 'id, D, I>
    where
        D::Ptr: SafeTransmute<D::Output>,
        I: IteratorDescription<'borrow, 'id, D>,
    {
        let mut lock = Default::default();
        let mut len = 0;
        let mut step = 0;

        VectorIterator {
            ptr: unsafe {
                I::LOCK(
                    self.scm.as_ptr(),
                    &raw mut lock,
                    &raw mut len,
                    &raw mut step,
                )
            },
            lock,
            len: NonZeroUsize::new(len),
            step: NonZeroIsize::new(step),
            _marker: PhantomData,
        }
    }

    pub fn val_iter<'borrow>(&'borrow self) -> ValIter<'borrow, 'id, D>
    where
        D::Ptr: SafeTransmute<D::Output>,
    {
        self.make_iter()
    }
    pub fn iter<'borrow>(&'borrow self) -> Iter<'borrow, 'id, D>
    where
        D::Ptr: SafeTransmute<D::Output>,
    {
        self.make_iter()
    }
    pub fn iter_mut<'borrow>(&'borrow mut self) -> IterMut<'borrow, 'id, D>
    where
        D::Ptr: SafeTransmute<D::Output>,
    {
        self.make_iter()
    }
}
impl<'id, D> Clone for Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<'id, D> Copy for Vector<'id, D> where D: VectorDescription<'id> {}
impl<'id, D> From<List<'id, D::Output>> for Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn from(list: List<'id, D::Output>) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr(D::FROM_LIST(list.pair.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'id, D> From<Vector<'id, D>> for List<'id, D::Output>
where
    D: VectorDescription<'id>,
{
    fn from(vector: Vector<'id, D>) -> Self {
        Self {
            pair: unsafe { Scm::from_ptr(D::TO_LIST(vector.scm.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'id, D> ScmTy<'id> for Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn type_name() -> Cow<'static, CStr> {
        D::type_name()
    }
    fn construct(self) -> Scm<'id> {
        self.scm
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        D::predicate(scm)
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

pub trait Ptr: Copy {
    fn is_null(self) -> bool;
    unsafe fn offset(self, _: isize) -> Self;
}
impl<T> Ptr for *const T {
    fn is_null(self) -> bool {
        self.is_null()
    }
    unsafe fn offset(self, count: isize) -> Self {
        unsafe { self.offset(count) }
    }
}
impl<T> Ptr for *mut T {
    fn is_null(self) -> bool {
        self.is_null()
    }
    unsafe fn offset(self, count: isize) -> Self {
        unsafe { self.offset(count) }
    }
}

pub struct VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
    lock: scm_t_array_handle,
    len: Option<NonZeroUsize>,
    step: Option<NonZeroIsize>,
    ptr: I::Ptr,
    _marker: PhantomData<(V, I)>,
}
impl<'borrow, 'id, V, I> DoubleEndedIterator for VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), ptr) if !ptr.is_null() => {
                self.len = NonZeroUsize::new(len.get() - 1);

                let offset = step.get() * isize::try_from(len.get() - 1).unwrap();
                let ptr = unsafe { ptr.offset(offset) };
                I::deref(ptr)
            }
            _ => None,
        }
    }
}
impl<'borrow, 'id, V, I> Drop for VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
    fn drop(&mut self) {
        unsafe {
            scm_array_handle_release(&raw mut self.lock);
        }
    }
}
impl<'borrow, 'id, V, I> ExactSizeIterator for VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
}

impl<'borrow, 'id, V, I> FusedIterator for VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
}
impl<'borrow, 'id, V, I> Iterator for VectorIterator<'borrow, 'id, V, I>
where
    V: VectorDescription<'id>,
    V::Ptr: SafeTransmute<V::Output>,
    I: IteratorDescription<'borrow, 'id, V>,
{
    type Item = I::Output;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.len, self.step, self.ptr) {
            (Some(len), Some(step), ptr) if !ptr.is_null() => {
                self.len = NonZeroUsize::new(len.get() - 1);
                self.ptr = unsafe { ptr.offset(step.get()) };
                I::deref(ptr)
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len.map(NonZeroUsize::get).unwrap_or_default();
        (len, Some(len))
    }
}

pub trait IteratorDescription<'borrow, 'id, D>
where
    D: VectorDescription<'id>,
    D::Ptr: SafeTransmute<D::Output>,
    D::Output: 'borrow,
{
    type Ptr: Ptr;
    type Output;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> Self::Ptr;

    fn deref(_: Self::Ptr) -> Option<Self::Output>;
}

pub struct ValIterDescription;
impl<'borrow, 'id, D> IteratorDescription<'borrow, 'id, D> for ValIterDescription
where
    D: VectorDescription<'id>,
    D::Ptr: SafeTransmute<D::Output>,
    D::Output: 'borrow,
{
    type Ptr = *const D::Ptr;
    type Output = D::Output;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> Self::Ptr = D::LOCK;

    fn deref(ptr: Self::Ptr) -> Option<Self::Output> {
        unsafe { ptr.cast::<D::Output>().as_ref() }.copied()
    }
}

pub type ValIter<'borrow, 'id, D> = VectorIterator<'borrow, 'id, D, ValIterDescription>;

pub struct IterDescription;
impl<'borrow, 'id, D> IteratorDescription<'borrow, 'id, D> for IterDescription
where
    D: VectorDescription<'id>,
    D::Ptr: SafeTransmute<D::Output>,
    D::Output: 'borrow,
{
    type Ptr = *const D::Ptr;
    type Output = &'borrow D::Output;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> Self::Ptr = D::LOCK;

    fn deref(ptr: Self::Ptr) -> Option<Self::Output> {
        unsafe { ptr.cast::<D::Output>().as_ref() }
    }
}

pub type Iter<'borrow, 'id, D> = VectorIterator<'borrow, 'id, D, IterDescription>;

pub struct IterMutDescription;
impl<'borrow, 'id, D> IteratorDescription<'borrow, 'id, D> for IterMutDescription
where
    D: VectorDescription<'id>,
    D::Ptr: SafeTransmute<D::Output>,
    D::Output: 'borrow,
{
    type Ptr = *mut D::Ptr;
    type Output = &'borrow mut D::Output;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> Self::Ptr = D::LOCK_MUT;

    fn deref(ptr: Self::Ptr) -> Option<Self::Output> {
        unsafe { ptr.cast::<D::Output>().as_mut() }
    }
}

pub type IterMut<'borrow, 'id, D> = VectorIterator<'borrow, 'id, D, IterMutDescription>;

/// # Safety
///
/// [Self::predicate] should dictate whether or not it is safe to run [Vector::get_unchecked] on that [Scm] object.
pub unsafe trait VectorDescription<'id> {
    /// If the type name of the vector can be known at compile time it should be placed here.
    fn type_name() -> Cow<'static, CStr>;

    // type Ptr: SafeTransmute<Self::Output>;
    type Ptr;
    type Output: ScmTy<'id>;

    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self::Ptr;
    const LOCK_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self::Ptr;

    fn predicate(_: &Scm) -> bool;
}

struct RawScmVectorDescription;
unsafe impl<'id> VectorDescription<'id> for RawScmVectorDescription {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"scm")
    }

    type Ptr = SCM;
    type Output = Scm<'id>;

    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_vector_to_list;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self::Ptr = crate::sys::scm_vector_elements;
    const LOCK_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self::Ptr = crate::sys::scm_vector_writable_elements;

    fn predicate(scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(crate::sys::scm_vector_p(scm.as_ptr())) }.is_true()
    }
}

pub type ScmVector<'id, T> = Vector<'id, ScmVectorDescription<'id, T>>;
// where
//     T: ScmTy<'id>;

pub struct ScmVectorDescription<'id, T>
where
    T: ScmTy<'id>,
{
    _marker: PhantomData<&'id T>,
}
unsafe impl<'id, T> VectorDescription<'id> for ScmVectorDescription<'id, T>
where
    T: ScmTy<'id>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "#{}()",
            BStr::new(T::type_name().as_ref().to_bytes())
        ))
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed(c"#()"))
    }

    type Ptr = SCM;
    type Output = T;

    const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_vector;
    const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = crate::sys::scm_vector_to_list;

    const LOCK: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *const Self::Ptr = crate::sys::scm_vector_elements;
    const LOCK_MUT: unsafe extern "C" fn(
        _: SCM,
        _: *mut scm_t_array_handle,
        _: *mut usize,
        _: *mut isize,
    ) -> *mut Self::Ptr = crate::sys::scm_vector_writable_elements;

    fn predicate(scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(crate::sys::scm_vector_p(scm.as_ptr())) }.is_true() && {
            Vector::<RawScmVectorDescription> {
                scm: *scm,
                _marker: PhantomData,
            }
            .iter()
            .all(|i| {
                let api = unsafe { Api::new_unchecked() };
                T::predicate(&api, &unsafe { i.cast_lifetime() })
            })
        }
    }
}

macro_rules! srfi_4_vector {
    ($rename:ident, $struct:ident, $predicate:expr, $from_list:expr, $to_list:expr, $lock:expr, $lock_mut:expr, $ty:ty $(,)?) => {
        srfi_4_vector!(
            $rename,
            $struct,
            $predicate,
            $from_list,
            $to_list,
            $lock,
            $lock_mut,
            $ty,
            stringify!($ty)
        );
    };
    ($rename:ident, $struct:ident, $predicate:expr, $from_list:expr, $to_list:expr, $lock:expr, $lock_mut:expr, $ty:ty, $ty_name:expr $(,)?) => {
        pub struct $struct;
        unsafe impl<'id> VectorDescription<'id> for $struct {
            fn type_name() -> Cow<'static, CStr> {
                Cow::Borrowed(
                    match CStr::from_bytes_with_nul(concat!("#", $ty_name, "()\0").as_bytes()) {
                        Ok(name) => name,
                        Err(_) => unreachable!(),
                    },
                )
            }

            type Ptr = $ty;
            type Output = $ty;

            const FROM_LIST: unsafe extern "C" fn(_: SCM) -> SCM = $from_list;
            const TO_LIST: unsafe extern "C" fn(_: SCM) -> SCM = $to_list;

            const LOCK: unsafe extern "C" fn(
                _: SCM,
                _: *mut scm_t_array_handle,
                _: *mut usize,
                _: *mut isize,
            ) -> *const Self::Ptr = $lock;
            const LOCK_MUT: unsafe extern "C" fn(
                _: SCM,
                _: *mut scm_t_array_handle,
                _: *mut usize,
                _: *mut isize,
            ) -> *mut Self::Ptr = $lock_mut;

            fn predicate(scm: &Scm) -> bool {
                unsafe { Scm::from_ptr(($predicate)(scm.as_ptr())) }.is_true()
            }
        }

        pub type $rename<'id> = Vector<'id, $struct>;
    };
}
srfi_4_vector!(
    U8Vector,
    U8VectorDescription,
    crate::sys::scm_u8vector_p,
    crate::sys::scm_list_to_u8vector,
    crate::sys::scm_u8vector_to_list,
    crate::sys::scm_u8vector_elements,
    crate::sys::scm_u8vector_writable_elements,
    u8,
);
srfi_4_vector!(
    U16Vector,
    U16VectorDescription,
    crate::sys::scm_u16vector_p,
    crate::sys::scm_list_to_u16vector,
    crate::sys::scm_u16vector_to_list,
    crate::sys::scm_u16vector_elements,
    crate::sys::scm_u16vector_writable_elements,
    u16,
);
srfi_4_vector!(
    U32Vector,
    U32VectorDescription,
    crate::sys::scm_u32vector_p,
    crate::sys::scm_list_to_u32vector,
    crate::sys::scm_u32vector_to_list,
    crate::sys::scm_u32vector_elements,
    crate::sys::scm_u32vector_writable_elements,
    u32
);
srfi_4_vector!(
    U64Vector,
    U64VectorDescription,
    crate::sys::scm_u64vector_p,
    crate::sys::scm_list_to_u64vector,
    crate::sys::scm_u64vector_to_list,
    crate::sys::scm_u64vector_elements,
    crate::sys::scm_u64vector_writable_elements,
    u64,
);
srfi_4_vector!(
    I8Vector,
    I8VectorDescription,
    crate::sys::scm_s8vector_p,
    crate::sys::scm_list_to_s8vector,
    crate::sys::scm_s8vector_to_list,
    crate::sys::scm_s8vector_elements,
    crate::sys::scm_s8vector_writable_elements,
    i8,
    "s8",
);
srfi_4_vector!(
    I16Vector,
    I16VectorDescription,
    crate::sys::scm_s16vector_p,
    crate::sys::scm_list_to_s16vector,
    crate::sys::scm_s16vector_to_list,
    crate::sys::scm_s16vector_elements,
    crate::sys::scm_s16vector_writable_elements,
    i16,
    "s16",
);
srfi_4_vector!(
    I32Vector,
    I32VectorDescription,
    crate::sys::scm_s32vector_p,
    crate::sys::scm_list_to_s32vector,
    crate::sys::scm_s32vector_to_list,
    crate::sys::scm_s32vector_elements,
    crate::sys::scm_s32vector_writable_elements,
    i32,
    "s32",
);
srfi_4_vector!(
    I64Vector,
    I64VectorDescription,
    crate::sys::scm_s64vector_p,
    crate::sys::scm_list_to_s64vector,
    crate::sys::scm_s64vector_to_list,
    crate::sys::scm_s64vector_elements,
    crate::sys::scm_s64vector_writable_elements,
    i64,
    "s64",
);

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn vector_iter() {
        with_guile(|api| {
            assert_eq!(
                I32Vector::from(api.make_list([1, 2, 3]))
                    .val_iter()
                    .collect::<Vec<i32>>(),
                [3, 2, 1]
            );
        })
        .unwrap();
    }
}
