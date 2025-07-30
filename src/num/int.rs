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
        num::{ExactIntegerTy, NumTy, RealTy},
        sys,
    },
    std::{borrow::Cow, ffi::CStr},
};

macro_rules! impl_scm_ty_for_int {
    ([ $(($ty:ty, $ty_name:literal, $ptr:ty, $predicate:expr, $to_scm:expr, $to_int:expr $(,)?)),+ $(,)? ]) => {
        $(impl_scm_ty_for_int!($ty, $ty_name, $ptr, $predicate, $to_scm, $to_int);)+
    };
    ($ty:ty, $ty_name:literal, $ptr:ty, $predicate:expr, $to_scm:expr, $to_int:expr) => {
        impl<'id> ScmTy<'id> for $ty {
            // SAFETY: this is in a const context and there is a null byte concatted at the end.
            fn type_name() -> Cow<'static, CStr> {
                const { Cow::Borrowed($ty_name) }
            }

            fn construct(self) -> Scm<'id> {
                unsafe { Scm::from_ptr(($to_scm)(self)) }
            }
            fn predicate(_: &Api, scm: &Scm) -> bool {
                unsafe {
                    ($predicate)(
                        scm.as_ptr(),
                        <$ty>::MIN as $ptr,
                        <$ty>::MAX as $ptr,
                    )
                }
            }
            unsafe fn get_unchecked(_: &Api, scm: Scm) -> Self {
                unsafe { ($to_int)(scm.as_ptr()) }
            }
        }
        impl ExactIntegerTy<'_> for $ty {}
        impl NumTy<'_> for $ty {}
        impl RealTy<'_> for $ty {}
    };
}
impl_scm_ty_for_int!([
    (
        i8,
        c"s8",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int8,
        sys::scm_to_int8
    ),
    (
        i16,
        c"s16",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int16,
        sys::scm_to_int16
    ),
    (
        i32,
        c"s32",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int32,
        sys::scm_to_int32
    ),
    (
        u8,
        c"u8",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint8,
        sys::scm_to_uint8
    ),
    (
        u16,
        c"u16",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint16,
        sys::scm_to_uint16
    ),
    (
        u32,
        c"u32",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint32,
        sys::scm_to_uint32
    ),
    (
        i64,
        c"s64",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int64,
        sys::scm_to_int64,
    ),
    (
        u64,
        c"u64",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint64,
        sys::scm_to_uint64,
    ),
]);
#[cfg(target_pointer_width = "32")]
impl_scm_ty_for_int!([
    (
        isize,
        c"s32",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_intptr_t,
        sys::scm_to_intptr_t
    ),
    (
        usize,
        c"u32",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uintptr_t,
        sys::scm_to_uintptr_t
    ),
]);
#[cfg(target_pointer_width = "64")]
impl_scm_ty_for_int!([
    (
        isize,
        c"s64",
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_intptr_t,
        sys::scm_to_intptr_t
    ),
    (
        usize,
        c"u64",
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uintptr_t,
        sys::scm_to_uintptr_t
    ),
]);

#[cfg(test)]
mod tests {
    use crate::{test_real, with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_conversion() {
        with_guile(|api| {
            test_real!(api, [i8, i16, i32, i64, isize, u8, u16, u32, u64, usize]);
        })
        .unwrap();
    }
}
