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
        scm::{Scm, TryFromScm},
        sys::SCM,
    },
    std::{
        marker::PhantomData,
        mem,
        ops::{Deref, DerefMut},
    },
};

/// Marker trait for types that are `repr(transparent)` to a [SCM] pointer.
pub unsafe trait ReprScm {}

pub struct Ref<'a, 'gm, T> {
    ptr: SCM,
    _marker: PhantomData<&'a &'gm T>,
}
impl<'a, 'gm, T> Clone for Ref<'_, 'gm, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'gm, T> Copy for Ref<'_, 'gm, T> {}
impl<'gm, T> Ref<'_, 'gm, T> {
    /// # Safety
    ///
    /// `ptr` must be able to safely converted to `T` through [TryFromScm::from_scm_unchecked], where the inner type operates on the [SCM].
    pub unsafe fn new_unchecked(ptr: SCM) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    pub fn into_inner(self) -> T
    where
        T: Copy + for<'a> TryFromScm<'a>,
    {
        let guile = unsafe { Guile::new_unchecked() };
        let ptr = Scm::from_ptr(self.ptr, &guile);
        T::try_from_scm(ptr, &guile).unwrap()
    }
}
impl<T> Deref for Ref<'_, '_, T>
where
    T: ReprScm,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

#[repr(transparent)]
pub struct RefMut<'a, 'gm, T>(Ref<'a, 'gm, T>);
impl<'gm, T> RefMut<'_, 'gm, T> {
    /// # Safety
    ///
    /// See [Ref::new_unchecked].
    /// `ptr` must also not be aliased.
    pub unsafe fn new_unchecked(ptr: SCM) -> Self {
        Self(unsafe { Ref::new_unchecked(ptr) })
    }

    pub fn into_inner(self) -> T
    where
        T: Copy + for<'a> TryFromScm<'a>,
    {
        self.0.into_inner()
    }
}
impl<T> Deref for RefMut<'_, '_, T>
where
    T: ReprScm,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<T> DerefMut for RefMut<'_, '_, T>
where
    T: ReprScm,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { mem::transmute(self) }
    }
}
