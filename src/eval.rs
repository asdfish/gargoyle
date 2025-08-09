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

use crate::{
    Guile,
    module::Module,
    reference::ReprScm,
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
    /// # use garguile::{module::Module, string::String, symbol::Symbol, with_guile};
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
            scm_primitive_load(path.as_ptr());
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
            Scm::from_ptr(unsafe { scm_eval_string(str.as_ptr()) }, self),
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
                unsafe { scm_eval_string_in_module(str.as_ptr(), module.as_ptr()) },
                self,
            ),
            self,
        )
    }
}
