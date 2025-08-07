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
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        sys::scm_procedure_p,
        utils::scm_predicate,
    },
    std::{borrow::Cow, ffi::CStr},
};

#[repr(transparent)]
pub struct Proc<'gm>(Scm<'gm>);
unsafe impl ReprScm for Proc<'_> {}
impl<'gm> TryFromScm<'gm> for Proc<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"procedure")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_procedure_p(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self(scm)
    }
}
impl<'gm> ToScm<'gm> for Proc<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.0
    }
}

pub trait GuileFn {
    fn create<'gm>(_: &'gm Guile) -> Proc<'gm>;
}

/// Create a struct and implement [GuileFn] for it.
///
/// The function requires everything to be behind references.
///
/// # Arguments
///
/// Arguments are passed with the syntax `#[guile_fn($KEY = $VAL)]` where KEY is the argument being set and VAL is the argument value.
///
/// | name | description | type |
/// | - | - | - |
/// | `guile_ident` | Identifier of the function used in metadata. Defaults to the name of the function but in kebab case | [c string literal][CStr] |
/// | `struct_ident` | The identifier used to implement [GuileFn]. Defaults to the name of the function but in pascal case | identfier |
/// | `gargoyle_root` | The path to the `gargoyle` crate. This is useful if you renamed the crate. | path |
///
/// # Examples
///
/// ```
/// # use gargoyle::{module::Module, string::String, subr::{guile_fn, GuileFn}, symbol::Symbol, with_guile};
/// #[guile_fn]
/// /// Add 2 numbers.
/// fn add(l: &i32, r: &i32) -> i32 {
///     *l + *r
/// }
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Module::current(guile).define(Symbol::from_str("add", guile), Add::create(guile));
///     assert_eq!(unsafe { String::from_str("(add 1 2)", guile).eval::<i32>() }, Ok(3));
/// }).unwrap();
/// ```
///
/// ```
/// # use gargoyle::{subr::guile_fn, subr::GuileFn};
/// #[guile_fn(guile_ident = c"is-even?", struct_ident = EvenPredicate)]
/// fn is_even(i: &i32) -> bool {
///     *i % 2 == 0
/// }
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, module::Module, reference::Ref, string::String, subr::{GuileFn, guile_fn}, symbol::Symbol, with_guile};
/// #[guile_fn]
/// fn sum(init: &i32, #[rest] r: &List<i32>) -> i32 {
///     r.iter().map(Ref::copied).fold(*init, |accum, r| accum + r)
/// }
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Module::current(guile).define(Symbol::from_str("sum", guile), Sum::create(guile));
///     assert_eq!(unsafe { String::from_str("(sum 1 2 3)", guile).eval::<i32>() }, Ok(6));
/// }).unwrap();
/// ```
///
/// ```
/// # use gargoyle::{Guile, collections::list::List, string::String, subr::{GuileFn, guile_fn}};
/// #[guile_fn]
/// fn length_string<'a>(#[guile] guile: &'a Guile, lst: &List<'a, i32>) -> String<'a> {
///     String::from_str(&lst.iter().count().to_string(), guile)
/// }
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, module::Module, string::String, subr::{GuileFn, guile_fn}, symbol::Symbol, with_guile};
/// #[guile_fn]
/// fn sub(l: &i32, #[optional] r: Option<&i32>) -> i32 {
///     if let Some(r) = r {
///         *l - *r
///     } else {
///         -*l
///     }
/// }
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Module::current(guile).define(Symbol::from_str("sub", guile), Sub::create(guile));
///     assert_eq!(unsafe { String::from_str("(sub 2 1)", guile).eval::<i32>() }, Ok(1));
///     assert_eq!(unsafe { String::from_str("(sub 1)", guile).eval::<i32>() }, Ok(-1));
/// }).unwrap();
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, module::Module, string::String, subr::{GuileFn, guile_fn}, symbol::Symbol, with_guile};
/// #[guile_fn]
/// fn area(#[keyword] width: Option<&i32>, height: Option<&i32>) -> i32 {
///     width.and_then(|width| height.map(|height| *width * *height)).unwrap_or_default()
/// }
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Module::current(guile).define(Symbol::from_str("area", guile), Area::create(guile));
///     assert_eq!(unsafe { String::from_str("(area #:width 10 #:height 10)", guile).eval::<i32>() }, Ok(100));
///     assert_eq!(unsafe { String::from_str("(area #:width 10)", guile).eval::<i32>() }, Ok(0));
/// }).unwrap();
/// ```
pub use proc_macros::guile_fn;
