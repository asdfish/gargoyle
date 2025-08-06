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

// FIXME: for some reason running this test with the usual test harness makes it hang, probably something to do with multithreading.

//! Implementation of traits for tuples
//!
//! # Examples
//! ```
//! # use gargoyle::{subr::{GuileFn, guile_fn}, string::String, with_guile};
//! #[guile_fn(guile_ident = c"sum-u8-i32")]
//! fn sum_u8_i32((l, r): &(u8, i32)) -> i32 {
//!     i32::from(*l) + *r
//! }
//!
//! # #[cfg(not(miri))]
//! with_guile(|guile| {
//!     SumU8I32::define_fn(guile);
//!     assert_eq!(
//!         unsafe { String::from_str("(sum-u8-i32 '(10 -10))", guile).eval::<i32>() },
//!         Ok(0)
//!     );
//! })
//! .unwrap();
//! ```

use crate::{list, scm::Scm};

macro_rules! cons_ty {
    () => {
        $crate::collections::list::Null
    };
    ($car:ty $(, $($cdr:ty),+ $(,)?)?) => {
        $crate::collections::pair::Pair<$car, cons_ty!($($($cdr),+)?)>
    };
}
macro_rules! impl_tuple {
    () => {};
    ($car:ident $(, $($cdr:ident),+ $(,)?)?) => {
        impl<'gm, $car, $($($cdr),*)?> $crate::scm::ToScm<'gm> for ($car, $( $($cdr),+)?)
        where
            $car: $crate::scm::ToScm<'gm>,
            $($($cdr: $crate::scm::ToScm<'gm>),+)?
        {
            fn to_scm(self, guile: &'gm $crate::Guile) -> Scm<'gm> {
                #[expect(non_snake_case)]
                let ($car, $( $($cdr),+)?) = self;

                let lst: $crate::collections::list::List<Scm<'gm>> = list!(
                    guile,
                    <$car as $crate::scm::ToScm>::to_scm($car, guile),
                    $($(<$cdr as $crate::scm::ToScm>::to_scm($cdr, guile)),+)?
                );
                lst.to_scm(guile)
            }
        }
        impl<'gm, $car, $($($cdr),*)?> $crate::scm::TryFromScm<'gm> for ($car, $( $($cdr),+)?)
        where
            $car: $crate::scm::TryFromScm<'gm>,
            $($($cdr: $crate::scm::TryFromScm<'gm>),+)?
        {
            fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
                #[allow(unused_macros)]
                macro_rules! add_string {
                    ($fst:literal $drop:tt) => { $fst };
                }
                ::std::ffi::CString::new(format!(
                    concat!("'(", "{}", $($(add_string!(" " $cdr), add_string!("{}" $cdr),)+)? ")"),
                    $crate::reexports::bstr::BStr::new(<$car as $crate::scm::TryFromScm>::type_name().as_ref().to_bytes()),
                    $($($crate::reexports::bstr::BStr::new(<$cdr as $crate::scm::TryFromScm>::type_name().as_ref().to_bytes()),)+)?
                ))
                    .map(::std::borrow::Cow::Owned)
                    .unwrap_or(::std::borrow::Cow::Borrowed(c"list"))
            }

            fn predicate(scm: &$crate::scm::Scm<'gm>, guile: &'gm $crate::Guile) -> bool {
                <cons_ty!($car $(, $($cdr),+)?)>::predicate(scm, guile)
            }

            #[expect(non_snake_case)]
            unsafe fn from_scm_unchecked(scm: $crate::scm::Scm<'gm>, guile: &'gm $crate::Guile) -> Self {
                let mut iter = unsafe { $crate::collections::list::List::<$crate::scm::Scm>::from_scm_unchecked(scm, guile) }
                .into_iter();

                let $car = iter.next().map(|scm| unsafe { $car::from_scm_unchecked(scm, guile) }).unwrap();
                $($(let $cdr = iter.next().map(|scm| unsafe { $cdr::from_scm_unchecked(scm, guile) }).unwrap();)+)?

                ($car, $($($cdr),+)?)
            }
        }

        impl_tuple!($($($cdr),*)?);
    };
}
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
