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
    std::ffi::CStr,
};

pub mod complex;
mod int;
pub mod rational;

/// Marker trait for types that pass `real?`
pub trait Real: ScmTy {}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Number<'id>(Scm<'id>);
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
impl Number<'_> {
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

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{Scm, num::Number, with_guile},
    };

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
}
