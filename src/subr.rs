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
        string::String,
        symbol::Symbol,
        sys::{
            scm_c_define_gsubr, scm_c_make_gsubr, scm_procedure_p, scm_set_procedure_property_x,
            scm_unused_struct,
        },
        utils::scm_predicate,
    },
    std::{
        borrow::Cow,
        ffi::{CStr, c_void},
        sync::{
            LazyLock,
            atomic::{self, AtomicPtr},
        },
    },
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

/// # Safety
///
/// [Self::ADDR] must be a pointer to a `unsafe extern "C"` function with an arity of `Self::REQUIRED + Self::OPTIONAL + Self::REST as usize` and can only be called in guile mode.
pub unsafe trait GuileFn {
    const ADDR: *mut c_void;

    const REQUIRED: usize;
    const OPTIONAL: usize;
    const REST: bool;

    const DOC: Option<&'static str>;
    const NAME: &'static CStr;

    fn make_fn<'gm>(guile: &'gm Guile) -> Proc<'gm> {
        Proc(Scm::from_ptr(
            unsafe {
                scm_c_make_gsubr(
                    Self::NAME.as_ptr(),
                    Self::REQUIRED.try_into().unwrap(),
                    Self::OPTIONAL.try_into().unwrap(),
                    Self::REST.into(),
                    Self::ADDR,
                )
            },
            guile,
        ))
    }

    fn define_fn<'gm>(guile: &'gm Guile) -> Proc<'gm> {
        Proc(Scm::from_ptr(
            unsafe {
                scm_c_define_gsubr(
                    Self::NAME.as_ptr(),
                    Self::REQUIRED.try_into().unwrap(),
                    Self::OPTIONAL.try_into().unwrap(),
                    Self::REST.into(),
                    Self::ADDR,
                )
            },
            guile,
        ))
    }

    /// Add documentation and potentially other attributes to a function.
    fn make_rich(proc: &mut Proc) {
        if let Some(docs) = Self::DOC {
            static DOCUMENTATION: LazyLock<AtomicPtr<scm_unused_struct>> = LazyLock::new(|| {
                let guile = unsafe { Guile::new_unchecked_ref() };
                Symbol::from_str("documentation", guile).ptr.into()
            });
            let guile = unsafe { Guile::new_unchecked_ref() };
            unsafe {
                scm_set_procedure_property_x(
                    proc.0.as_ptr(),
                    DOCUMENTATION.load(atomic::Ordering::SeqCst),
                    String::from_str(docs, guile).scm.as_ptr(),
                );
            }
        }
    }
}

/// Create a struct and implement [GuileFn] for it.
///
/// The function requires everything to be behind references since the arguments can be aliased. It does assume that the object is not being mutated in other threads but that would need to be unsafe anyways.
///
/// To make functions safe to use in unsafe scheme code, you should use immutable data.
///
/// # Arguments
///
/// Arguments are passed with the syntax `#[guile_fn($KEY = $VAL)]` where KEY is the argument being set and VAL is the argument value.
///
/// | name | description | type |
/// | - | - | - |
/// | `doc` | The string used in [GuileFn::DOC]. If unset, default to the function's doc comments. | [String literal][str] or [false]. [false] will set [GuileFn::DOC] to [None] |
/// | `guile_ident` | The identifier used in [GuileFn::NAME]. Defaults to the name of the function but in kebab case | [c string literal][CStr] |
/// | `struct_ident` | The identifier used to implement [GuileFn]. Defaults to the name of the function but in pascal case | identfier |
/// | `gargoyle_root` | The path to the `gargoyle` crate. This is useful if you renamed the crate. | path |
///
/// # Examples
///
/// ```
/// # use gargoyle::{string::String, subr::{guile_fn, GuileFn}, with_guile};
/// #[guile_fn]
/// /// Add 2 numbers.
/// fn add(l: &i32, r: &i32) -> i32 {
///     *l + *r
/// }
/// assert_eq!(Add::REQUIRED, 2);
/// assert_eq!(Add::OPTIONAL, 0);
/// assert_eq!(Add::REST, false);
/// assert_eq!(Add::NAME, c"add");
/// assert_eq!(Add::DOC, Some(" Add 2 numbers."));
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Add::define_fn(guile);
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
/// assert_eq!(EvenPredicate::REQUIRED, 1);
/// assert_eq!(EvenPredicate::OPTIONAL, 0);
/// assert_eq!(EvenPredicate::REST, false);
/// assert_eq!(EvenPredicate::NAME, c"is-even?");
/// assert_eq!(EvenPredicate::DOC, None);
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, reference::Ref, string::String, subr::{GuileFn, guile_fn}, with_guile};
/// #[guile_fn]
/// fn sum(init: &i32, #[rest] r: &List<i32>) -> i32 {
///     r.iter().map(Ref::into_inner).fold(*init, |accum, r| accum + r)
/// }
/// assert_eq!(Sum::REQUIRED, 1);
/// assert_eq!(Sum::OPTIONAL, 0);
/// assert_eq!(Sum::REST, true);
/// assert_eq!(Sum::NAME, c"sum");
/// assert_eq!(Sum::DOC, None);
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Sum::define_fn(guile);
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
/// assert_eq!(LengthString::REQUIRED, 1);
/// assert_eq!(LengthString::OPTIONAL, 0);
/// assert_eq!(LengthString::REST, false);
/// assert_eq!(LengthString::NAME, c"length-string");
/// assert_eq!(LengthString::DOC, None);
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, string::String, subr::{GuileFn, guile_fn}, with_guile};
/// #[guile_fn]
/// fn sub(l: &i32, #[optional] r: Option<&i32>) -> i32 {
///     if let Some(r) = r {
///         *l - *r
///     } else {
///         -*l
///     }
/// }
/// assert_eq!(Sub::REQUIRED, 1);
/// assert_eq!(Sub::OPTIONAL, 1);
/// assert_eq!(Sub::REST, false);
/// assert_eq!(Sub::NAME, c"sub");
/// assert_eq!(Sub::DOC, None);
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Sub::define_fn(guile);
///     assert_eq!(unsafe { String::from_str("(sub 2 1)", guile).eval::<i32>() }, Ok(1));
///     assert_eq!(unsafe { String::from_str("(sub 1)", guile).eval::<i32>() }, Ok(-1));
/// }).unwrap();
/// ```
///
/// ```
/// # use gargoyle::{collections::list::List, string::String, subr::{GuileFn, guile_fn}, with_guile};
/// #[guile_fn]
/// fn area(#[keyword] width: Option<&i32>, height: Option<&i32>) -> i32 {
///     width.and_then(|width| height.map(|height| *width * *height)).unwrap_or_default()
/// }
/// assert_eq!(Area::REQUIRED, 0);
/// assert_eq!(Area::OPTIONAL, 0);
/// assert_eq!(Area::REST, true);
/// assert_eq!(Area::NAME, c"area");
/// assert_eq!(Area::DOC, None);
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     Area::define_fn(guile);
///     assert_eq!(unsafe { String::from_str("(area #:width 10 #:height 10)", guile).eval::<i32>() }, Ok(100));
///     assert_eq!(unsafe { String::from_str("(area #:width 10)", guile).eval::<i32>() }, Ok(0));
/// }).unwrap();
/// ```
pub use proc_macros::guile_fn;
