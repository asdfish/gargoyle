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
        string::String,
        sys::{
            scm_c_symbol_length, scm_eq_p, scm_from_utf8_symboln, scm_make_symbol,
            scm_string_to_symbol, scm_symbol_interned_p, scm_symbol_p, scm_symbol_to_string,
        },
    },
    std::{borrow::Cow, ffi::CStr},
};

impl Api {
    pub fn make_symbol<'id, S>(&'id self, sym: &S) -> Symbol<'id>
    where
        S: AsRef<str> + ?Sized,
    {
        let sym = sym.as_ref();
        Symbol(unsafe {
            Scm::from_ptr(scm_from_utf8_symboln(
                sym.as_bytes().as_ptr().cast(),
                sym.len(),
            ))
        })
    }

    // pub fn take_symbol<'id>(&'id self, sym: String<Vec<u8, CAllocator>>) -> Symbol<'id> {
    //     let (ptr, len, _) = sym.into_inner().into_raw_parts();
    //     Symbol(unsafe { Scm::from_ptr(scm_take_utf8_symboln(ptr.cast(), len)) })
    // }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Symbol<'id>(Scm<'id>);
impl<'id> Symbol<'id> {
    pub fn len(&self) -> usize {
        unsafe { scm_c_symbol_length(self.0.as_ptr()) }
    }

    /// Create an uninterned symbol.
    ///
    /// Uninterned symbols will never be equal to anything else.
    pub fn new_uninterned(string: String<'id>) -> Self {
        Self(unsafe { Scm::from_ptr(scm_make_symbol(string.0.as_ptr())) })
    }

    pub fn is_interned(&self) -> bool {
        unsafe { Scm::from_ptr(scm_symbol_interned_p(self.0.as_ptr())) }.is_true()
    }
}
impl<'id> From<Symbol<'id>> for String<'id> {
    fn from(sym: Symbol<'id>) -> String<'id> {
        String(unsafe { Scm::from_ptr(scm_symbol_to_string(sym.0.as_ptr())) })
    }
}
impl<'id> From<String<'id>> for Symbol<'id> {
    fn from(string: String<'id>) -> Symbol<'id> {
        Symbol(unsafe { Scm::from_ptr(scm_string_to_symbol(string.0.as_ptr())) })
    }
}
impl PartialEq for Symbol<'_> {
    fn eq(&self, r: &Self) -> bool {
        unsafe { Scm::from_ptr(scm_eq_p(self.0.as_ptr(), r.0.as_ptr())) }.is_true()
    }
}
impl<'id> ScmTy<'id> for Symbol<'id> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"symbol")
    }

    fn construct(self) -> Scm<'id> {
        self.0
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_symbol_p(scm.as_ptr())) }.is_true()
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self(scm)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn make_sym() {
        with_guile(|api| {
            assert_eq!(
                api.make_symbol("foo"),
                api.eval_c(c"'foo").get::<Symbol>().unwrap(),
            );
            assert_eq!(String::from(api.make_symbol("foo")), api.make_string("foo"),);
            assert_eq!(Symbol::from(api.make_string("bar")), api.make_symbol("bar"),);

            assert!(api.make_symbol("foo").is_interned());
            assert!(!Symbol::new_uninterned(api.make_string("bar")).is_interned());
            assert_ne!(
                Symbol::new_uninterned(api.make_string("bar")),
                api.make_symbol("bar"),
            );
            assert_ne!(
                Symbol::new_uninterned(api.make_string("bar")),
                Symbol::new_uninterned(api.make_string("bar")),
            );
        })
        .unwrap();
    }
}
