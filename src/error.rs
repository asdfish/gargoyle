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

use {
    crate::{Guile, collections::list::List, scm::ToScm, sys::scm_misc_error},
    std::ffi::CStr,
};

impl Guile {
    ///
    pub fn misc_error<'gm, T>(&'gm self, subr: &CStr, msg: &CStr, list: List<'gm, T>) -> !
    where
        T: ToScm<'gm>,
    {
        unsafe {
            scm_misc_error(subr.as_ptr(), msg.as_ptr(), list.to_scm(self).as_ptr());
        }
        unreachable!()
    }
}
