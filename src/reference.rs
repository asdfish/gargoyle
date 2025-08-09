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

//! References to data in collections.

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
///
/// # Safety
///
/// Implementing types must be `repr(transparent)` to a [SCM] pointer.
pub unsafe trait ReprScm {
    /// # Safety
    ///
    /// You must check the type of the scm.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{reference::ReprScm, sys::SCM};
    /// # use std::ptr;
    /// #[repr(transparent)]
    /// struct Scm(SCM);
    /// unsafe impl ReprScm for Scm {}
    /// let scm = unsafe { Scm::from_ptr(ptr::dangling_mut()) };
    /// ```
    unsafe fn from_ptr(scm: SCM) -> Self
    where
        Self: Sized,
    {
        unsafe { mem::transmute_copy(&scm) }
    }

    /// Get the pointer out of something.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{reference::ReprScm, sys::SCM};
    /// # use std::ptr;
    /// #[repr(transparent)]
    /// struct Scm(SCM);
    /// unsafe impl ReprScm for Scm {}
    /// let ptr = Scm(ptr::dangling_mut()).as_ptr();
    /// ```
    fn as_ptr(&self) -> SCM
    where
        Self: Sized,
    {
        unsafe { mem::transmute_copy::<Self, SCM>(self) }
    }
}

/// Reference created with a [Scm].
#[derive(Debug)]
#[repr(transparent)]
pub struct Ref<'a, 'gm, T> {
    ptr: SCM,
    _marker: PhantomData<&'a &'gm T>,
}
impl<'gm, T> Clone for Ref<'_, 'gm, T> {
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

    /// Copy the data from the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(Pair::new(0, 1, guile).as_car().copied(), 0);
    /// }).unwrap();
    /// ```
    pub fn copied(self) -> T
    where
        T: Copy + TryFromScm<'gm>,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let ptr = Scm::from_ptr(self.ptr, guile);
        T::try_from_scm(ptr, guile).unwrap()
    }
}
impl<'a, 'gm, T> Ref<'a, 'gm, T> {
    /// # Safety
    ///
    /// The lifetime `'a` must be the lifetime of the pointer
    pub(crate) unsafe fn from_ptr(ptr: SCM) -> Result<Self, Ref<'a, 'gm, Scm<'gm>>>
    where
        T: TryFromScm<'gm>,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        if T::predicate(&Scm::from_ptr(ptr, guile), guile) {
            Ok(Self {
                ptr,
                _marker: PhantomData,
            })
        } else {
            Err(Ref {
                ptr,
                _marker: PhantomData,
            })
        }
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

/// Mutable reference created with a [Scm].
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

    /// See [Ref::copied]
    pub fn copied(self) -> T
    where
        T: Copy + TryFromScm<'gm>,
    {
        self.0.copied()
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

#[cfg(test)]
mod tests {
    use {super::*, std::ptr};

    #[test]
    fn deref() {
        #[repr(transparent)]
        struct TestObj(SCM);
        unsafe impl ReprScm for TestObj {}
        impl TestObj {
            fn addr(&self) -> usize {
                self.0.addr()
            }
            fn set_ptr(&mut self, ptr: SCM) {
                self.0 = ptr;
            }
        }

        let null = ptr::null_mut();
        let r = unsafe { Ref::<TestObj>::new_unchecked(null) };
        assert_eq!(r.addr(), null.addr());

        let null = ptr::null_mut();
        let mut r = unsafe { RefMut::<TestObj>::new_unchecked(null) };
        assert_eq!(r.addr(), null.addr());
        let ptr = ptr::dangling_mut();
        r.set_ptr(ptr);
        assert_eq!(r.addr(), ptr.addr());
    }

    #[test]
    fn from_ptr() {
        #[repr(transparent)]
        struct Foo(SCM);
        unsafe impl ReprScm for Foo {}

        unsafe {
            Foo::from_ptr(ptr::null_mut());
        }
    }
}
