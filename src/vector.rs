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
    crate::{Api, Scm, ScmTy, list::List, sys::scm_vector},
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
};

/// Marker traits for types that may be safely transmuted
pub unsafe trait SafeTransmute<T>
where
    Self: Sized,
    T: Sized,
{
}
// SAFETY: all types should be transmutable to themself
unsafe impl<T> SafeTransmute<T> for T {}
// SAFETY: these types are `#[repr(transparent)]` onto [Scm].
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::char_set::CharSet<'id> {}
unsafe impl<'id, T> SafeTransmute<Scm<'id>> for crate::list::List<'id, T> where T: ScmTy<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::complex::Complex<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::rational::Rational<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::ExactInteger<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::num::Number<'id> {}
unsafe impl<'id> SafeTransmute<Scm<'id>> for crate::string::String<'id> {}

#[derive(Debug)]
#[repr(transparent)]
pub struct Vector<'id, D>
where
    D: VectorDescription<'id> + ?Sized,
{
    scm: Scm<'id>,
    _marker: PhantomData<&'id D>,
}
impl<'id, D> From<List<'id, D::Output>> for Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn from(list: List<'id, D::Output>) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr(scm_vector(list.pair.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'id, D> ScmTy<'id> for Vector<'id, D>
where
    D: VectorDescription<'id>,
{
    fn type_name() -> Cow<'static, CStr> {
        D::TYPE_NAME
            .map(Cow::Borrowed)
            .unwrap_or_else(<D::Output as ScmTy<'id>>::type_name)
    }
    fn construct(self) -> Scm<'id> {
        self.scm
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        D::predicate(scm)
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

unsafe trait VectorIteratorDescription {
    type Ptr;
    type Output;
}

/// # Safety
///
/// [Self::predicate] should dictate whether or not it is safe to run [Vector::get_unchecked] on that [Scm] object.
pub unsafe trait VectorDescription<'id> {
    /// If the type name of the vector can be known at compile time it should be placed here.
    const TYPE_NAME: Option<&'static CStr>;

    type Ptr: SafeTransmute<Self::Output>;
    type Output: ScmTy<'id>;

    fn predicate(scm: &Scm) -> bool;
}

macro_rules! srfi_4_vector {
    ($rename:ident, $struct:ident, $predicate:expr, $ty:ty $(,)?) => {
        srfi_4_vector!($rename, $struct, $predicate, $ty, stringify!($ty));
    };
    ($rename:ident, $struct:ident, $predicate:expr, $ty:ty, $ty_name:expr $(,)?) => {
        pub struct $struct;
        unsafe impl<'id> VectorDescription<'id> for $struct {
            const TYPE_NAME: Option<&'static CStr> = Some(
                match CStr::from_bytes_with_nul(concat!("#", $ty_name, "()\0").as_bytes()) {
                    Ok(name) => name,
                    Err(_) => unreachable!(),
                },
            );

            type Ptr = $ty;
            type Output = $ty;

            fn predicate(scm: &Scm) -> bool {
                unsafe { Scm::from_ptr(($predicate)(scm.as_ptr())) }.is_true()
            }
        }

        pub type $rename<'id> = Vector<'id, $struct>;
    };
}
srfi_4_vector!(
    U8Vector,
    U8VectorDescription,
    crate::sys::scm_u8vector_p,
    u8,
);
srfi_4_vector!(
    U16Vector,
    U16VectorDescription,
    crate::sys::scm_u16vector_p,
    u16,
);
srfi_4_vector!(
    U32Vector,
    U32VectorDescription,
    crate::sys::scm_u32vector_p,
    u32
);
srfi_4_vector!(
    U64Vector,
    U64VectorDescription,
    crate::sys::scm_u64vector_p,
    u64,
);
srfi_4_vector!(
    I8Vector,
    I8VectorDescription,
    crate::sys::scm_s8vector_p,
    i8,
    "s8",
);
srfi_4_vector!(
    I16Vector,
    I16VectorDescription,
    crate::sys::scm_s16vector_p,
    i16,
    "s16",
);
srfi_4_vector!(
    I32Vector,
    I32VectorDescription,
    crate::sys::scm_s32vector_p,
    i32,
    "s32",
);
srfi_4_vector!(
    I64Vector,
    I64VectorDescription,
    crate::sys::scm_s64vector_p,
    i64,
    "s64",
);
