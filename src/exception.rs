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
        sys::scm_wrong_type_arg_msg,
    },
    std::{convert::Infallible, ffi::CStr, marker::PhantomData},
};

pub trait Exception<'guile_mode> {
    fn throw(self, _: &'guile_mode Guile) -> !
    where
        Self: Sized;
}

impl Exception<'_> for Infallible {
    fn throw(self, _: &Guile) -> ! {
        unreachable!()
    }
}
pub struct WrongTypeArg<'gm, T, E>
where
    T: ToScm<'gm>,
    E: TryFromScm<'gm>,
{
    subr: &'static CStr,
    arg: usize,
    val: T,
    _marker: PhantomData<&'gm E>,
}
impl<'gm, T, E> WrongTypeArg<'gm, T, E>
where
    T: ToScm<'gm>,
    E: TryFromScm<'gm>,
{
    pub fn new(subr: &'static CStr, arg: usize, val: T) -> Self {
        Self {
            subr,
            arg,
            val,
            _marker: PhantomData,
        }
    }
}
impl<'gm, T, E> Exception<'gm> for WrongTypeArg<'gm, T, E>
where
    T: ToScm<'gm>,
    E: TryFromScm<'gm>,
{
    fn throw(self, g: &'gm Guile) -> ! {
        unsafe {
            scm_wrong_type_arg_msg(
                self.subr.as_ptr(),
                self.arg.try_into().unwrap(),
                self.val.to_scm(g).as_ptr(),
                E::type_name().as_ref().as_ptr(),
            );
        }
        unreachable!()
    }
}
