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
    crate::{Api, Scm, ScmTy, sys},
    std::{
        cmp::Ordering,
        ffi::CStr,
        ops::{Add, Div, Mul, Rem, Sub},
    },
};

pub mod complex;
mod int;
pub mod rational;

/// Marker trait for types that always pass `num?`
pub trait Num: ScmTy {}
/// Marker trait for types that pass `real?`
pub trait Real: Num {}

impl Api {
    pub fn make_num<'id, T>(&'id self, num: T) -> Number<'id>
    where
        T: Num,
    {
        Number(self.make(num))
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Number<'id>(Scm<'id>);
impl<'id> Number<'id> {
    pub fn into_scm(self) -> Scm<'id> {
        self.0
    }

    pub fn is_real(&self) -> bool {
        unsafe { sys::scm_is_real(self.0.as_ptr()) }
    }
    pub fn is_exact(&self) -> bool {
        unsafe { sys::scm_is_exact(self.0.as_ptr()) }
    }
    pub fn is_inexact(&self) -> bool {
        unsafe { sys::scm_is_inexact(self.0.as_ptr()) }
    }
    pub fn make_exact(&self) -> Self {
        unsafe { Self(Scm::from_ptr(sys::scm_inexact_to_exact(self.0.as_ptr()))) }
    }
    pub fn make_inexact(&self) -> Self {
        unsafe { Self(Scm::from_ptr(sys::scm_exact_to_inexact(self.0.as_ptr()))) }
    }
}
impl ScmTy for Number<'_> {
    type Output = Self;

    const TYPE_NAME: &'static CStr = c"number";

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { self.0.cast_lifetime() }
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_number(scm.as_ptr()) }
    }

    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        Self(unsafe { scm.cast_lifetime() })
    }
}
impl Num for Number<'_> {}

macro_rules! impl_op_for_number {
    ($op:ident, $fn:ident, $scm_fn:expr) => {
        impl<R> $op<R> for Number<'_>
        where
            R: Num,
        {
            type Output = Self;

            fn $fn(self, r: R) -> Self {
                let api = unsafe { Api::new_unchecked() };
                let r = api.make(r);
                Self(unsafe { Scm::from_ptr(($scm_fn)(self.0.as_ptr(), r.as_ptr())) })
            }
        }
    };
}
impl_op_for_number!(Add, add, sys::scm_sum);
impl_op_for_number!(Sub, sub, sys::scm_difference);
// This will throw with divide by zero, but rust already does that.
impl_op_for_number!(Div, div, sys::scm_divide);
impl_op_for_number!(Rem, rem, sys::scm_remainder);
impl_op_for_number!(Mul, mul, sys::scm_product);
impl<R> PartialEq<R> for Number<'_>
where
    R: Clone + Copy + Num,
{
    fn eq(&self, r: &R) -> bool {
        let api = unsafe { Api::new_unchecked() };
        let r = api.make(*r);
        unsafe { Scm::from_ptr(sys::scm_num_eq_p(self.0.as_ptr(), r.as_ptr())).is_true() }
    }
}
impl<R> PartialOrd<R> for Number<'_>
where
    R: Clone + Copy + Num,
{
    fn partial_cmp(&self, r: &R) -> Option<Ordering> {
        let api = unsafe { Api::new_unchecked() };
        let r = api.make_num(*r);
        (self.is_real() && r.is_real())
            .then(|| {
                [
                    (
                        sys::scm_less_p
                            as unsafe extern "C" fn(_: sys::SCM, _: sys::SCM) -> sys::SCM,
                        Ordering::Less,
                    ),
                    (sys::scm_num_eq_p, Ordering::Equal),
                    (sys::scm_gr_p, Ordering::Greater),
                ]
                .into_iter()
                .find_map(|(predicate, output)| {
                    unsafe { Scm::from_ptr((predicate)(self.0.as_ptr(), r.0.as_ptr())) }
                        .is_true()
                        .then_some(output)
                })
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Scm, num::Number, with_guile};

    pub fn assert_numberness(scm: Scm) {
        assert!(
            scm.get::<Number<'_>>()
                .map(|num| num.is_real())
                .unwrap_or_default()
        );
    }

    #[macro_export]
    macro_rules! test_real {
        ($api:expr, [ $($ty:ty),+ $(,)? ]) => {
            $(test_real!($api, $ty);)+
        };
        ($api:expr, $ty:ty) => {
            let scm = <$crate::Api as $crate::tests::ApiExt>::test_real_equal($api, <$ty>::MIN);
            $crate::num::tests::assert_numberness(scm);
            let scm = <$crate::Api as $crate::tests::ApiExt>::test_real_equal($api, <$ty>::MAX);
            $crate::num::tests::assert_numberness(scm);
        };
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn exactness() {
        with_guile(|api| {
            let num = api.make(5.0).get::<Number<'_>>().unwrap();
            assert!(num.is_inexact());
            let num = num.make_exact();
            assert!(num.is_exact());
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn math() {
        with_guile(|api| {
            assert_eq!(api.make_num(1) + 2, 3);
            assert_eq!(api.make_num(1) - 2, -1);
            assert_eq!(api.make_num(5) * 2, 10);
            assert_eq!(api.make_num(8) / 2, 4);
            assert_eq!(api.make_num(10) % 3, 1);
        })
        .unwrap()
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_ord() {
        with_guile(|api| {
            let [ref one, ref two, ref three] =
                (1..=3).map(|i| api.make_num(i)).collect::<Vec<_>>()[..]
            else {
                unreachable!()
            };

            assert!(one < two);
            assert!(one < three);
            assert!(one <= one);
            assert!(one <= two);
            assert!(one <= three);
            assert!(three > one);
            assert!(three > two);
            assert!(three >= one);
            assert!(three >= two);
            assert!(three >= three);
        })
        .unwrap();
    }
}
