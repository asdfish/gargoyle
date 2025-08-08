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

use crate::{
    Guile,
    module::Module,
    scm::{Scm, TryFromScm},
    string::String,
    sys::{scm_eval_string, scm_eval_string_in_module, scm_primitive_load},
};

impl Guile {
    /// # Safety
    ///
    /// Ensure the file doesn't do anything unsafe.
    ///
    /// # Examples
    /// ```
    /// # use gargoyle::{module::Module, string::String, symbol::Symbol, with_guile};
    /// # use std::{io::Write as _, str};
    /// # use tempfile::NamedTempFile;
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut file = NamedTempFile::new().unwrap();
    ///     write!(file.as_file_mut(), "(define my-var 69)").unwrap();
    ///     let path = String::from_str(str::from_utf8(file.path().as_os_str().as_encoded_bytes()).unwrap(), guile);
    ///     unsafe { guile.load_path(path); }
    ///     assert_eq!(Module::current(guile).read::<i32>(Symbol::from_str("my-var", guile)).unwrap().unwrap().copied(), 69);
    /// }).unwrap();
    /// ```
    pub unsafe fn load_path<'gm>(&'gm self, path: String<'gm>) {
        unsafe {
            scm_primitive_load(path.scm.as_ptr());
        }
    }

    /// # Safety
    ///
    /// Since you can do very unsafe things in scheme, there is probably no way to make this safe.
    ///
    /// # Exceptions
    ///
    /// This might also potentially throw an exception if the string is not correct.
    pub unsafe fn eval<'gm, T>(&'gm self, str: &String<'gm>) -> Result<T, Scm<'gm>>
    where
        T: TryFromScm<'gm>,
    {
        T::try_from_scm(
            Scm::from_ptr(unsafe { scm_eval_string(str.scm.as_ptr()) }, self),
            self,
        )
    }

    /// # Safety
    ///
    /// See [Self::eval].
    pub unsafe fn eval_in<'gm, T>(
        &'gm self,
        str: &String<'gm>,
        module: &Module<'gm>,
    ) -> Result<T, Scm<'gm>>
    where
        T: TryFromScm<'gm>,
    {
        T::try_from_scm(
            Scm::from_ptr(
                unsafe { scm_eval_string_in_module(str.scm.as_ptr(), module.0.as_ptr()) },
                self,
            ),
            self,
        )
    }
}
