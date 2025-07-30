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
        borrow::Cow,
        cmp::Ordering,
        ffi::CStr,
        ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Rem, Sub},
    },
};

pub mod complex;
mod int;
pub mod rational;

/// Marker trait for types that always pass `num?`
pub trait NumTy<'id>: ScmTy<'id> {}
/// Marker trait for types that pass `real?`
pub trait RealTy<'id>: NumTy<'id> {}
/// Marker trait for types that pass `exact-integer?`
pub trait ExactIntegerTy<'id>: NumTy<'id> {}

impl Api {
    pub fn make_num<'id, 'b, T>(&'id self, num: T) -> Number<'id>
    where
        T: NumTy<'b>,
    {
        Number(self.make(num))
    }
    pub fn make_exact<'id, 'b, T>(&'id self, num: T) -> ExactInteger<'id>
    where
        T: ExactIntegerTy<'b>,
    {
        ExactInteger(self.make(num))
    }
}

trait ScmNum<'id>: NumTy<'id> {
    unsafe fn as_ptr(&self) -> sys::SCM;
    fn is_real(&self) -> bool;
}
macro_rules! impl_scm_num {
    ($num:ty) => {
        impl<'id, R> PartialEq<R> for $num
        where
            R: Clone + Copy + NumTy<'id>,
        {
            fn eq(&self, r: &R) -> bool {
                let r = R::construct(*r);
                unsafe { Scm::from_ptr(sys::scm_num_eq_p(self.as_ptr(), r.as_ptr())).is_true() }
            }
        }

        impl<'id, R> PartialOrd<R> for $num
        where
            R: Clone + Copy + NumTy<'id>,
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
                            unsafe { Scm::from_ptr((predicate)(self.as_ptr(), r.0.as_ptr())) }
                                .is_true()
                                .then_some(output)
                        })
                    })
                    .flatten()
            }
        }
        impl_op_for_scm_num!($num, Add, add, sys::scm_sum);
        impl_op_for_scm_num!($num, Sub, sub, sys::scm_difference);
        impl_op_for_scm_num!($num, Div, div, sys::scm_divide);
        impl_op_for_scm_num!($num, Rem, rem, sys::scm_remainder);
        impl_op_for_scm_num!($num, Mul, mul, sys::scm_product);
    };
}
macro_rules! impl_op_for_scm_num {
    ($ty:ty, $op:ident, $fn:ident, $scm_fn:expr) => {
        impl<'id, R> $op<R> for $ty
        where
            R: NumTy<'id>,
        {
            type Output = Number<'id>;

            fn $fn(self, r: R) -> Self::Output {
                let api = unsafe { Api::new_unchecked() };
                let r = api.make(r);
                Number(unsafe { Scm::from_ptr(($scm_fn)(self.as_ptr(), r.as_ptr())) })
            }
        }
    };
}
impl_scm_num!(complex::Complex<'id>);
impl_scm_num!(rational::Rational<'id>);

#[derive(Debug)]
#[repr(transparent)]
pub struct ExactInteger<'id>(Scm<'id>);
impl<'id> ExactIntegerTy<'id> for ExactInteger<'id> {}
impl<'id> NumTy<'id> for ExactInteger<'id> {}
impl<'id> RealTy<'id> for ExactInteger<'id> {}
impl<'id> ScmNum<'id> for ExactInteger<'id> {
    unsafe fn as_ptr(&self) -> sys::SCM {
        unsafe { self.0.as_ptr() }
    }

    fn is_real(&self) -> bool {
        true
    }
}
impl<'id> ScmTy<'id> for ExactInteger<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"exact integer")
    }
    fn construct(self) -> Scm<'id> {
        unsafe { self.0.cast_lifetime() }
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_exact_integer(scm.as_ptr()) }
    }

    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self(scm)
    }
}
impl_scm_num!(ExactInteger<'id>);
impl_op_for_scm_num!(ExactInteger<'id>, BitAnd, bitand, sys::scm_logand);
impl_op_for_scm_num!(ExactInteger<'id>, BitOr, bitor, sys::scm_logior);
impl_op_for_scm_num!(ExactInteger<'id>, BitXor, bitxor, sys::scm_logxor);

#[derive(Debug)]
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
impl<'id> ScmNum<'id> for Number<'id> {
    unsafe fn as_ptr(&self) -> sys::SCM {
        unsafe { self.0.as_ptr() }
    }
    fn is_real(&self) -> bool {
        unsafe { sys::scm_is_real(self.0.as_ptr()) }
    }
}
impl<'id> ScmTy<'id> for Number<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"number")
    }

    fn construct(self) -> Scm<'id> {
        unsafe { self.0.cast_lifetime() }
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_number(scm.as_ptr()) }
    }

    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self(scm)
    }
}
impl<'id> NumTy<'id> for Number<'id> {}
impl_scm_num!(Number<'id>);

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

            assert_eq!(api.make_exact(1) + 2, 3);
            assert_eq!(api.make_exact(1) - 2, -1);
            assert_eq!(api.make_exact(5) * 2, 10);
            assert_eq!(api.make_exact(8) / 2, 4);
            assert_eq!(api.make_exact(10) % 3, 1);

            assert_eq!(api.make_exact(0b11) & 0b01, 0b01);
            assert_eq!(api.make_exact(0b11) | 0b10, 0b11);
            assert_eq!(api.make_exact(0b11) ^ 0b10, 0b01);
        })
        .unwrap()
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_ord() {
        with_guile(|api| {
            let [ref one, _, ref three] = (1..=3).map(|i| api.make_num(i)).collect::<Vec<_>>()[..]
            else {
                unreachable!()
            };

            assert!(one < &2);
            assert!(one < &3);
            assert!(one <= &1);
            assert!(one <= &2);
            assert!(one <= &3);
            assert!(three > &1);
            assert!(three > &2);
            assert!(three >= &1);
            assert!(three >= &2);
            assert!(three >= &3);
        })
        .unwrap();
    }
}
