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

macro_rules! impl_scm_traits_for_int {
    ($ty:ty, $ty_name:literal,
     $scm_is_int:expr, $ptr:ty, $scm_to_int:expr, $scm_from_int:expr $(,)?) => {
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
#[cfg(target_pointer_width = "32")]
mod bits32 {
    impl_scm_traits_for_int!(
        usize,
        "u32",
        crate::sys::scm_is_unsigned_integer,
        usize,
        crate::sys::scm_to_uintptr_t,
        crate::sys::scm_from_uintptr_t,
    );
    impl_scm_traits_for_int!(
        isize,
        "s32",
        crate::sys::scm_is_signed_integer,
        isize,
        crate::sys::scm_to_intptr_t,
        crate::sys::scm_from_intptr_t,
    );
}
#[cfg(target_pointer_width = "64")]
mod bits64 {
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
        "u64",
        crate::sys::scm_is_unsigned_integer,
        usize,
        crate::sys::scm_to_uintptr_t,
        crate::sys::scm_from_uintptr_t,
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
        "s64",
        crate::sys::scm_is_signed_integer,
        isize,
        crate::sys::scm_to_intptr_t,
        crate::sys::scm_from_intptr_t,
    );
}

macro_rules! impl_scm_traits_for_float {
    ($ty:ty) => {
        impl<'gm> $crate::scm::TryFromScm<'gm> for $ty {
            fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
                const {
                    ::std::borrow::Cow::Borrowed(unsafe {
                        ::std::ffi::CStr::from_bytes_with_nul_unchecked(
                            concat!(stringify!($ty), "\0").as_bytes(),
                        )
                    })
                }
            }

            fn predicate(scm: &$crate::scm::Scm<'gm>, _: &'gm $crate::Guile) -> bool {
                $crate::utils::c_predicate(|| unsafe { $crate::sys::scm_is_real(scm.as_ptr()) })
            }
            unsafe fn from_scm_unchecked(
                scm: $crate::scm::Scm<'gm>,
                _: &'gm $crate::Guile,
            ) -> Self {
                let float = unsafe { $crate::sys::scm_to_double(scm.as_ptr()) };
                if float <= <$ty>::MAX as f64 {
                    float as $ty
                } else {
                    <$ty>::INFINITY
                }
            }
        }
    };
}
impl_scm_traits_for_float!(f32);
impl_scm_traits_for_float!(f64);
