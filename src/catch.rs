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
    crate::{Api, CallbackData, Scm, sys::SCM},
    std::ffi::c_void,
};

/// # Safety
///
/// `ptr` must be a pointer of type `CallbackData<F, Result<O, Exception>>`
unsafe extern "C" fn callback<F, O>(ptr: *mut c_void) -> SCM
where
    F: FnOnce(&Api) -> O,
{
    let ptr = ptr.cast::<CallbackData<F, Result<O, Exception>>>();
    if let Some(data) = unsafe { ptr.as_mut() } {
        if data.output.is_none() {
            let api = unsafe { Api::new_unchecked() };
            data.output = data
                .operation
                .take()
                .map(|operation| operation(&api))
                .map(Ok);
        }
    }

    unsafe { crate::sys::SCM_UNDEFINED }
}

/// # Safety
/// See [callback]
unsafe extern "C" fn handler<F, O>(ptr: *mut c_void, key: SCM, args: SCM) -> SCM
where
    F: FnOnce(&Api) -> O,
{
    let ptr = ptr.cast::<CallbackData<F, Result<O, Exception<'static>>>>();
    if let Some(data) = unsafe { ptr.as_mut() } {
        if data.output.is_none() {
            data.output = Some(Err(Exception {
                key: unsafe { Scm::from_ptr(key) },
                args: unsafe { Scm::from_ptr(args) },
            }));
        }
    }

    unsafe { crate::sys::SCM_UNDEFINED }
}

impl Api {
    /// Catch exceptions while calling `thunk` if they are of type `tag`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::with_guile;
    /// #[cfg(not(miri))]
    /// with_guile(|api| {
    ///     let output = api.catch(api.make(true), |api| {
    ///         api.misc_error(c"main", c"intentionally threw here", api.make(()));
    ///     }).unwrap();
    ///     assert!(output.is_err());
    /// }).unwrap();
    /// ```
    pub fn catch<'id, F, O>(&'id self, tag: Scm, thunk: F) -> Option<Result<O, Exception<'id>>>
    where
        F: FnOnce(&Self) -> O,
    {
        let mut data = CallbackData::<F, Result<O, Exception<'id>>> {
            operation: Some(thunk),
            output: None,
        };
        let ptr = (&raw mut data).cast::<c_void>();
        unsafe {
            crate::sys::scm_internal_catch(
                tag.as_ptr(),
                Some(callback::<F, O>),
                ptr,
                Some(handler::<F, O>),
                ptr,
            )
        };

        data.output
    }

    pub fn catch_all<'id, F, O>(&'id self, thunk: F) -> Result<O, Exception<'id>>
    where
        F: FnOnce(&Self) -> O,
    {
        self.catch(self.make(true), thunk)
            .expect("`#t` should catch all exception")
    }
}

#[derive(Debug)]
pub struct Exception<'id> {
    pub key: Scm<'id>,
    pub args: Scm<'id>,
}
