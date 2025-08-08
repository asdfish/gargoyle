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

//! Guile equivalent of `Box<dyn Any>`

use {
    crate::{
        Guile,
        reference::ReprScm,
        sys::{
            SCM, SCM_UNBNDP, scm_equal_p, scm_is_false, scm_is_true, scm_null_p,
            scm_wrong_type_arg_msg,
        },
        utils::{c_predicate, scm_predicate},
    },
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
};

/// Trait for types that can be converted from a [Scm] object.
pub trait TryFromScm<'gm> {
    /// The name of the type
    fn type_name() -> Cow<'static, CStr>;

    /// Whether or not the object is this type
    fn predicate(_: &Scm<'gm>, _: &'gm Guile) -> bool;

    /// Try to convert the [Scm] to this type.
    fn try_from_scm(scm: Scm<'gm>, guile: &'gm Guile) -> Result<Self, Scm<'gm>>
    where
        Self: Sized,
    {
        if Self::predicate(&scm, guile) {
            Ok(unsafe { Self::from_scm_unchecked(scm, guile) })
        } else {
            Err(scm)
        }
    }

    /// Attempt to convert the type or throw an exception.
    fn from_scm_or_throw(scm: Scm<'gm>, proc: &CStr, idx: usize, guile: &'gm Guile) -> Self
    where
        Self: Sized,
    {
        Self::try_from_scm(scm, guile).unwrap_or_else(|scm| {
            unsafe {
                scm_wrong_type_arg_msg(
                    proc.as_ptr(),
                    idx.try_into().unwrap(),
                    scm.as_ptr(),
                    Self::type_name().as_ref().as_ptr(),
                );
            }
            unreachable!()
        })
    }

    /// Create [Self] without type checking.
    ///
    /// # Safety
    ///
    /// [Self::predicate] should implement type checking.
    unsafe fn from_scm_unchecked(_: Scm<'gm>, _: &'gm Guile) -> Self
    where
        Self: Sized;
}
pub use proc_macros::TryFromScm;

/// Trait for types that can be converted to a [Scm] object.
pub trait ToScm<'gm> {
    /// Convert this type to a [Scm]
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm>
    where
        Self: Sized;
}
/// Derive [ToScm].
///
/// # Examples
///
/// ```
/// # use gargoyle::{foreign_object::ForeignObject, module::Module, scm::{ToScm, TryFromScm}, string::String, subr::{guile_fn, GuileFn}, symbol::Symbol, with_guile};
/// # use std::sync::atomic::{self, AtomicBool};
/// #[derive(Clone, Copy, Debug, ForeignObject, PartialEq, ToScm, TryFromScm)]
/// struct Coordinate {
///     x: i32,
///     y: i32,
/// }
/// #[guile_fn]
/// fn make_coordinate(#[keyword] x: Option<&i32>, y: Option<&i32>) -> Coordinate {
///     Coordinate {
///         x: x.copied().unwrap_or_default(),
///         y: y.copied().unwrap_or_default(),
///     }
/// }
///
/// static CALLED: AtomicBool = AtomicBool::new(false);
/// #[guile_fn]
/// fn must_call(_: &Coordinate) -> bool {
///     CALLED.swap(true, atomic::Ordering::Release)
/// }
/// # #[cfg(not(miri))] {
/// with_guile(|guile| {
///     let mut module = Module::current(guile);
///     module.define(Symbol::from_str("must-call", guile), MustCall::create(guile));
///     module.define(Symbol::from_str("make-coordinate", guile), MakeCoordinate::create(guile));
///     assert_eq!(unsafe { guile.eval::<bool>(&String::from_str("(must-call (make-coordinate #:x 10 #:y 20))", guile)) }, Ok(false));
///     assert_eq!(unsafe { guile.eval::<Coordinate>(&String::from_str("(make-coordinate #:x 10)", guile)) }, Ok(Coordinate { x: 10, y: 0 }));
/// }).unwrap();
/// assert_eq!(CALLED.load(atomic::Ordering::Acquire), true);
/// # }
/// ```
pub use proc_macros::ToScm;

/// Guile equivalent of `Box<dyn Any>`
#[derive(Debug)]
#[repr(transparent)]
pub struct Scm<'gm> {
    pub(crate) ptr: SCM,
    _marker: PhantomData<&'gm ()>,
}
impl<'gm> Scm<'gm> {
    /// Create a [Scm] from a pointer and a lifetime.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{scm::Scm, with_guile};
    /// # use std::ptr;
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     // safe since the `Scm`'s lifetime is bound to the lifetime of `guile`
    ///     let scm = Scm::from_ptr(ptr::dangling_mut(), guile);
    /// }).unwrap();
    /// ```
    pub fn from_ptr(ptr: SCM, _: &'gm Guile) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
    /// # Safety
    ///
    /// The lifetime of the [Scm] object should be tied to a [Guile] so that it will always be in guile mode.
    pub unsafe fn from_ptr_unchecked(ptr: SCM) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    pub(crate) fn is_true(&self) -> bool {
        c_predicate(unsafe { scm_is_true(self.as_ptr()) })
    }
    pub(crate) fn is_false(&self) -> bool {
        c_predicate(unsafe { scm_is_false(self.as_ptr()) })
    }
    pub(crate) fn is_eol(&self) -> bool {
        scm_predicate(unsafe { scm_null_p(self.as_ptr()) })
    }

    /// # Safety
    ///
    /// Ensure the inner type may be cloned.
    pub unsafe fn copy_unchecked(&self) -> Self {
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}
impl PartialEq for Scm<'_> {
    /// Compare equality with `equal?`
    fn eq(&self, r: &Self) -> bool {
        unsafe { Scm::from_ptr_unchecked(scm_equal_p(self.as_ptr(), r.as_ptr())) }.is_true()
    }
}
unsafe impl ReprScm for Scm<'_> {}
impl<'gm> TryFromScm<'gm> for Scm<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"any")
    }

    fn predicate(_: &Scm<'gm>, _: &'gm Guile) -> bool {
        true
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        scm
    }
}
impl<'gm> ToScm<'gm> for Scm<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self
    }
}

impl<'gm, T> TryFromScm<'gm> for Option<T>
where
    T: TryFromScm<'gm>,
{
    fn type_name() -> Cow<'static, CStr> {
        T::type_name()
    }

    fn predicate(scm: &Scm<'gm>, guile: &'gm Guile) -> bool {
        c_predicate(unsafe { SCM_UNBNDP(scm.as_ptr()) }) || T::predicate(scm, guile)
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, guile: &'gm Guile) -> Self {
        if c_predicate(unsafe { SCM_UNBNDP(scm.as_ptr()) }) {
            None
        } else {
            Some(unsafe { T::from_scm_unchecked(scm, guile) })
        }
    }
}
