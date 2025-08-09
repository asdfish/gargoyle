// garguile - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Try catch facilities in rust.

use {
    crate::{
        Guile,
        collections::list::List,
        reference::ReprScm,
        scm::{Scm, TryFromScm},
        symbol::Symbol,
        sys::{SCM, SCM_BOOL_T, SCM_UNDEFINED, scm_internal_catch, scm_throw},
    },
    std::ffi::c_void,
};

/// Tag for the type of error that you would like to catch.
pub enum Tag<'gm> {
    /// Catch all symbols.
    All,
    /// Catch a specfic symbol.
    Symbol(Symbol<'gm>),
}

struct CallbackData<F, T> {
    thunk: Option<F>,
    output: Option<T>,
}

/// # Safety
///
/// `data` must be a pointer of type `CallbackData<F, T>`
unsafe extern "C" fn body_callback<'gm, F, T>(data: *mut c_void) -> SCM
where
    F: FnOnce(&'gm Guile) -> T,
{
    if let Some(CallbackData { thunk, output }) =
        unsafe { data.cast::<CallbackData<F, T>>().as_mut() }
    {
        *output = thunk
            .take()
            .map(|thunk| thunk(unsafe { Guile::new_unchecked_ref() }));
    }

    unsafe { SCM_UNDEFINED }
}

/// # Safety
///
/// `data` must be a pointer of type `CallbackData<F, T>`
unsafe extern "C" fn handler_callback<'a, F, T>(data: *mut c_void, key: SCM, args: SCM) -> SCM
where
    F: FnOnce(&'a Guile, Symbol<'a>, List<'a, Scm<'a>>) -> T,
{
    if let Some(CallbackData { thunk, output }) =
        unsafe { data.cast::<CallbackData<F, T>>().as_mut() }
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let key = Symbol::try_from_scm(Scm::from_ptr(key, guile), guile).unwrap();
        let args = List::try_from_scm(Scm::from_ptr(args, guile), guile).unwrap();

        *output = thunk.take().map(|thunk| thunk(guile, key, args));
    }

    unsafe { SCM_UNDEFINED }
}

impl Guile {
    /// Throw an exception and start unwinding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::list::List, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// assert!(with_guile(|guile| guile.throw(Symbol::from_str("foo", guile), List::<i32>::new(guile))).is_none());
    /// ```
    pub fn throw<'gm, T>(&'gm self, ty: Symbol<'gm>, args: List<'gm, T>) -> ! {
        unsafe {
            scm_throw(ty.as_ptr(), args.as_ptr());
        }

        unreachable!()
    }

    /// # Examples
    ///
    /// ```
    /// # use garguile::{catch::Tag, collections::list::List, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(guile.try_catch(Tag::All, |_| {}, |_, _, _| {}).is_ok());
    ///     assert!(guile.try_catch(Tag::All, |guile| guile.throw(Symbol::from_str("foo", guile), List::<i32>::new(guile)), |_, _, _| {}).is_err());
    ///     assert!(guile.try_catch(Tag::Symbol(Symbol::from_str("foo", guile)), |guile| guile.throw(Symbol::from_str("foo", guile), List::<i32>::new(guile)), |_, _, _| {}).is_err());
    /// }).unwrap();
    /// ```
    pub fn try_catch<'gm, B, H, T, E>(&'gm self, tag: Tag<'gm>, body: B, handler: H) -> Result<T, E>
    where
        B: FnOnce(&'gm Self) -> T,
        H: FnOnce(&'gm Self, Symbol<'gm>, List<'gm, Scm<'gm>>) -> E,
    {
        let mut body_data = CallbackData::<B, T> {
            thunk: Some(body),
            output: None,
        };
        let mut handler_data = CallbackData::<H, E> {
            thunk: Some(handler),
            output: None,
        };

        let tag = match tag {
            Tag::All => unsafe { SCM_BOOL_T },
            Tag::Symbol(symbol) => symbol.as_ptr(),
        };
        unsafe {
            scm_internal_catch(
                tag,
                Some(body_callback::<'gm, B, T>),
                (&raw mut body_data).cast(),
                Some(handler_callback::<'gm, H, E>),
                (&raw mut handler_data).cast(),
            );
        }

        body_data
            .output
            .map(Ok)
            .or_else(|| handler_data.output.map(Err))
            .expect(
                "`scm_internal_catch` should be calling either callbacks with non null pointers",
            )
    }
}
