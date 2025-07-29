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
        num::{Number, Real},
        sys::{
            scm_denominator, scm_from_double, scm_inf, scm_is_rational, scm_nan, scm_numerator,
            scm_rationalize, scm_to_double,
        },
    },
    std::ffi::{CStr, c_double},
};

impl Api {
    /// Create a `+nan`.
    pub fn nan<'id>(&'id self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_nan()) }
    }
    /// Create a `+inf`.
    pub fn inf<'id>(&'id self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_inf()) }
    }

    pub fn rationalize<'id, T, U>(&'id self, real: T, eps: U) -> Rational<'id>
    where
        T: Real,
        U: Real,
    {
        let real = real.construct(self);
        let eps = eps.construct(self);
        unsafe { Rational(Scm::from_ptr(scm_rationalize(real.as_ptr(), eps.as_ptr()))) }
    }
}

impl ScmTy for c_double {
    type Output = Self;

    const TYPE_NAME: &CStr = c"double";

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_from_double(self)) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        scm.get::<Number<'_>>()
            .map(|num| num.is_real())
            .unwrap_or_default()
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self {
        unsafe { scm_to_double(scm.as_ptr()) }
    }
}
impl Real for c_double {}

/// A rational number
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Rational<'id>(Scm<'id>);
impl ScmTy for Rational<'_> {
    type Output = Self;

    const TYPE_NAME: &'static CStr = c"rational";

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { self.0.cast_lifetime() }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { scm_is_rational(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        Self(unsafe { (*scm).cast_lifetime() })
    }
}
impl<'id> Rational<'id> {
    pub fn denominator(&self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_denominator(self.0.as_ptr())) }
    }
    pub fn numerator(&self) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_numerator(self.0.as_ptr())) }
    }
}
impl Real for Rational<'_> {}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{test_real, with_guile},
    };

    #[cfg_attr(miri, ignore)]
    #[test]
    fn double() {
        with_guile(|api| {
            test_real!(api, c_double);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn rationalize() {
        with_guile(|api| {
            let rational = api.rationalize(0.5, 0);
            assert_eq!(rational.denominator().get::<c_double>(), Some(2.0));
            assert_eq!(rational.numerator().get::<c_double>(), Some(1.0));
        })
        .unwrap()
    }
}
