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
        sys::{scm_from_double, scm_to_double},
    },
    std::ffi::{CStr, c_double},
};
impl Api {}

impl ScmTy for c_double {
    type Output = Self;

    const TYPE_NAME: &CStr = c"double";

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { Scm::from_ptr(scm_from_double(self)) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        scm.is_real_number()
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self {
        unsafe { scm_to_double(scm.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{test_ty, with_guile},
    };

    #[test]
    fn double() {
        with_guile(|api| {
            test_ty!(api, c_double);
        });
    }
}
