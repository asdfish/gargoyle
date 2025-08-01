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
        scm::{Scm, ToScm, TryFromScm},
        sys::{
            scm_c_imag_part, scm_c_make_rectangular, scm_c_real_part, scm_from_double, scm_is_real,
            scm_to_double,
        },
        utils::c_predicate,
    },
    std::marker::PhantomData,
};

/// # Safety
///
/// All implementors must be able to be used functions like [scm_sum][crate::sys::scm_sum].
pub(crate) unsafe trait Num<'guile_mode>:
    Copy + ToScm<'guile_mode> + TryFromScm<'guile_mode>
{
}

pub(crate) trait UInt<'gm>: Num<'gm> {}
impl UInt<'_> for u8 {}
impl UInt<'_> for u16 {}
impl UInt<'_> for u32 {}
impl UInt<'_> for u64 {}
impl UInt<'_> for usize {}

macro_rules! impl_scm_traits_for_int {
    ($ty:ty, $ty_name:literal,
     $scm_is_int:path, $ptr:ty, $scm_to_int:path, $scm_from_int:path $(,)?) => {
        impl<'gm> $crate::scm::TryFromScm<'gm> for $ty {
            fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
                ::std::borrow::Cow::Borrowed(
                    const {
                        unsafe {
                            ::std::ffi::CStr::from_bytes_with_nul_unchecked(
                                concat!($ty_name, "\0").as_bytes(),
                            )
                        }
                    },
                )
            }

            fn predicate(scm: &$crate::scm::Scm<'gm>, _: &'gm $crate::Guile) -> bool {
                $crate::utils::c_predicate(|| unsafe {
                    $scm_is_int(scm.as_ptr(), <$ty>::MIN as $ptr, <$ty>::MAX as $ptr)
                })
            }

            unsafe fn from_scm_unchecked(scm: $crate::scm::Scm<'gm>, _: &'gm $crate::Guile) -> Self
            where
                Self: ::std::marker::Sized,
            {
                unsafe { $scm_to_int(scm.as_ptr()) }
            }
        }

        impl<'gm> $crate::scm::ToScm<'gm> for $ty {
            fn to_scm(self, guile: &'gm $crate::Guile) -> $crate::scm::Scm<'gm> {
                $crate::scm::Scm::from_ptr(unsafe { $scm_from_int(self) }, guile)
            }
        }
        unsafe impl<'gm> $crate::num::Num<'gm> for $ty {}
    };
}
impl_scm_traits_for_int!(
    u8,
    "u8",
    crate::sys::scm_is_unsigned_integer,
    usize,
    crate::sys::scm_to_uint8,
    crate::sys::scm_from_uint8,
);
impl_scm_traits_for_int!(
    u16,
    "u16",
    crate::sys::scm_is_unsigned_integer,
    usize,
    crate::sys::scm_to_uint16,
    crate::sys::scm_from_uint16,
);
impl_scm_traits_for_int!(
    u32,
    "u32",
    crate::sys::scm_is_unsigned_integer,
    usize,
    crate::sys::scm_to_uint32,
    crate::sys::scm_from_uint32,
);
impl_scm_traits_for_int!(
    u64,
    "u64",
    crate::sys::scm_is_unsigned_integer,
    usize,
    crate::sys::scm_to_uint64,
    crate::sys::scm_from_uint64,
);
impl_scm_traits_for_int!(
    usize,
    "usize",
    crate::sys::scm_is_unsigned_integer,
    usize,
    crate::sys::scm_to_uintptr_t,
    crate::sys::scm_from_uintptr_t,
);
impl_scm_traits_for_int!(
    i8,
    "s8",
    crate::sys::scm_is_signed_integer,
    isize,
    crate::sys::scm_to_int8,
    crate::sys::scm_from_int8,
);
impl_scm_traits_for_int!(
    i16,
    "s16",
    crate::sys::scm_is_signed_integer,
    isize,
    crate::sys::scm_to_int16,
    crate::sys::scm_from_int16,
);
impl_scm_traits_for_int!(
    i32,
    "s32",
    crate::sys::scm_is_signed_integer,
    isize,
    crate::sys::scm_to_int32,
    crate::sys::scm_from_int32,
);
impl_scm_traits_for_int!(
    i64,
    "s64",
    crate::sys::scm_is_signed_integer,
    isize,
    crate::sys::scm_to_int64,
    crate::sys::scm_from_int64,
);
impl_scm_traits_for_int!(
    isize,
    "ssize",
    crate::sys::scm_is_signed_integer,
    isize,
    crate::sys::scm_to_intptr_t,
    crate::sys::scm_from_intptr_t,
);

impl<'gm> TryFromScm<'gm> for f64 {
    fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
        const { ::std::borrow::Cow::Borrowed(c"f64") }
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(|| unsafe { scm_is_real(scm.as_ptr()) })
    }
    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        unsafe { scm_to_double(scm.as_ptr()) }
    }
}
impl<'gm> ToScm<'gm> for f64 {
    fn to_scm(self, guile: &'gm Guile) -> Scm<'gm> {
        Scm::from_ptr(unsafe { scm_from_double(self.into()) }, guile)
    }
}
unsafe impl Num<'_> for f64 {}

macro_rules! impl_ops_for_num {
    ($ident:ident, $op:ident, $fn:ident, $bin_fn:path) => {
        impl<'gm, R> ::std::ops::$op<R> for $ident<'gm>
        where
            R: for<'a> $crate::num::Num<'a>,
        {
            type Output = $crate::num::Number<'gm>;

            fn $fn(self, r: R) -> Self::Output {
                // SAFETY: having a [Self] exist is proof of being in guile mode.
                let guile = unsafe { $crate::Guile::new_unchecked() };
                let l = self.to_scm(&guile).as_ptr();
                let r = r.to_scm(&guile).as_ptr();

                $crate::num::Number {
                    scm: unsafe { $bin_fn(l, r) },
                    _marker: ::std::marker::PhantomData,
                }
            }
        }
    };
}
macro_rules! define_num {
    ($ident:ident, $type_name:literal, $predicate:path) => {
        // Numbers can be aliased since you cannot mutate them.
        #[derive(Clone, Copy)]
        pub struct $ident<'guile_mode> {
            scm: $crate::sys::SCM,
            _marker: ::std::marker::PhantomData<&'guile_mode ()>,
        }
        impl<'gm> $crate::scm::TryFromScm<'gm> for $ident<'gm> {
            fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
                const {
                    ::std::borrow::Cow::Borrowed(unsafe {
                        ::std::ffi::CStr::from_bytes_with_nul_unchecked(
                            concat!($type_name, "\0").as_bytes(),
                        )
                    })
                }
            }

            fn predicate(scm: &$crate::scm::Scm<'gm>, _: &'gm $crate::Guile) -> bool {
                $crate::utils::c_predicate(|| unsafe { $predicate(scm.as_ptr()) })
            }
            unsafe fn from_scm_unchecked(
                scm: $crate::scm::Scm<'gm>,
                _: &'gm $crate::Guile,
            ) -> Self {
                $ident {
                    scm: scm.ptr,
                    _marker: ::std::marker::PhantomData,
                }
            }
        }
        impl<'gm> $crate::scm::ToScm<'gm> for $ident<'gm> {
            fn to_scm(self, guile: &'gm $crate::Guile) -> $crate::scm::Scm<'gm> {
                $crate::scm::Scm::from_ptr(self.scm, guile)
            }
        }
        unsafe impl<'gm> $crate::num::Num<'gm> for $ident<'gm> {}

        impl_ops_for_num!($ident, Add, add, $crate::sys::scm_sum);
        impl_ops_for_num!($ident, Sub, sub, $crate::sys::scm_difference);
        impl_ops_for_num!($ident, Mul, mul, $crate::sys::scm_product);
        impl_ops_for_num!($ident, Div, div, $crate::sys::scm_divide);

        impl<R> ::std::cmp::PartialEq<R> for $ident<'_>
        where
            R: for<'a> $crate::num::Num<'a>,
        {
            fn eq(&self, r: &R) -> bool {
                let guile = unsafe { $crate::Guile::new_unchecked() };
                $crate::utils::scm_predicate(|| unsafe {
                    $crate::sys::scm_num_eq_p(self.scm, r.to_scm(&guile).as_ptr())
                })
            }
        }
        impl<R> ::std::cmp::PartialOrd<R> for $ident<'_>
        where
            R: for<'a> $crate::num::Num<'a>,
        {
            fn partial_cmp(&self, r: &R) -> ::std::option::Option<::std::cmp::Ordering> {
                let guile = unsafe { Guile::new_unchecked() };
                if self == r {
                    ::std::option::Option::Some(::std::cmp::Ordering::Equal)
                } else if $crate::utils::scm_predicate(|| unsafe {
                    $crate::sys::scm_less_p(self.to_scm(&guile).as_ptr(), r.to_scm(&guile).as_ptr())
                }) {
                    ::std::option::Option::Some(::std::cmp::Ordering::Less)
                } else if $crate::utils::scm_predicate(|| unsafe {
                    $crate::sys::scm_gr_p(self.to_scm(&guile).as_ptr(), r.to_scm(&guile).as_ptr())
                }) {
                    ::std::option::Option::Some(::std::cmp::Ordering::Greater)
                } else {
                    ::std::option::Option::None
                }
            }
        }
    };
}

define_num!(Number, "number", crate::sys::scm_is_number);
define_num!(Real, "real", crate::sys::scm_is_real);
impl From<Real<'_>> for f64 {
    fn from(real: Real<'_>) -> f64 {
        // SAFETY: all reals are real
        unsafe { scm_to_double(real.scm) }
    }
}
define_num!(Rational, "rational", crate::sys::scm_is_rational);
impl From<Rational<'_>> for f64 {
    fn from(rat: Rational<'_>) -> f64 {
        // SAFETY: rationals are all real
        unsafe { scm_to_double(rat.scm) }
    }
}
define_num!(Complex, "complex", crate::sys::scm_is_complex);
impl Complex<'_> {
    pub fn real_part(&self) -> f64 {
        unsafe { scm_c_real_part(self.scm) }
    }
    pub fn imag_part(&self) -> f64 {
        unsafe { scm_c_imag_part(self.scm) }
    }
}
impl<'gm> Complex<'gm> {
    pub fn new(real: f64, imag: f64, _: &'gm Guile) -> Self {
        Self {
            scm: unsafe { scm_c_make_rectangular(real, imag) },
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn complex() {
        with_guile(|guile| {
            let complex = Complex::new(10.0, 30.0, &guile);
            assert_eq!(complex.real_part(), 10.0);
            assert_eq!(complex.imag_part(), 30.0);
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn ty_min_max() {
        macro_rules! test_ty {
            ($ty:ty) => {
                $crate::with_guile(|guile| {
                    assert_eq!(
                        <$ty as $crate::scm::TryFromScm>::try_from_scm(
                            <$ty as $crate::scm::ToScm>::to_scm(<$ty>::MIN, guile),
                            guile
                        ),
                        Ok(<$ty>::MIN),
                    );
                    assert_eq!(
                        <$ty as $crate::scm::TryFromScm>::try_from_scm(
                            <$ty as $crate::scm::ToScm>::to_scm(<$ty>::MAX, guile),
                            guile
                        ),
                        Ok(<$ty>::MAX),
                    );
                })
                .unwrap();
            };
        }
        test_ty!(i8);
        test_ty!(i16);
        test_ty!(i32);
        test_ty!(isize);
        test_ty!(u8);
        test_ty!(u16);
        test_ty!(u32);
        test_ty!(usize);
        #[cfg(target_pointer_width = "64")]
        {
            test_ty!(i64);
            test_ty!(u64);
        }
    }

    /// test that we can alias the pointers with [Copy]
    #[cfg_attr(miri, ignore)]
    #[test]
    fn aliasing() {
        with_guile(|guile| {
            let fst = Complex::new(10.0, 30.0, &guile);
            let snd = fst;
            let fst = fst + 10.0;

            assert_eq!(
                Complex::try_from_scm(fst.to_scm(&guile), &guile).map(|c| c.real_part()),
                Ok(20.0)
            );
            assert_eq!(snd.real_part(), 10.0);
        });
    }
}
