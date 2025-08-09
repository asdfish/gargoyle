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
    crate::sys::{SCM, scm_is_true},
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, c_int},
        fmt::{self, Display, Formatter},
    },
};

pub fn c_predicate(b: c_int) -> bool {
    b != 0
}

pub fn scm_predicate(b: SCM) -> bool {
    c_predicate(unsafe { scm_is_true(b) })
}

pub trait CowCStrExt<'a> {
    fn display(&'a self) -> CowCStrDisplay<'a>;
}
impl<'a> CowCStrExt<'a> for Cow<'a, CStr> {
    fn display(&'a self) -> CowCStrDisplay<'a> {
        CowCStrDisplay(self)
    }
}
pub struct CowCStrDisplay<'a>(&'a Cow<'a, CStr>);
impl<'a> Display for CowCStrDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        BStr::new(self.0.as_ref().to_bytes()).fmt(f)
    }
}
