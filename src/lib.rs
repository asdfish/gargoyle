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

//! Rust bindings to guile.

#![expect(private_bounds)]

pub mod alloc;
pub mod catch;
pub mod collections;
pub mod dynwind;
mod eval;
pub mod foreign_object;
mod guile_mode;
pub mod hook;
pub mod module;
pub mod num;
mod primitive;
#[doc(hidden)]
pub mod reexports;
pub mod reference;
pub mod scm;
pub mod string;
pub mod subr;
pub mod symbol;
pub mod sys;
mod utils;

use std::ptr::NonNull;

pub use guile_mode::*;

/// Token that proves the current thread is in guile mode.
#[repr(transparent)]
pub struct Guile {
    _marker: (),
}
impl Guile {
    /// # Safety
    ///
    /// This can be run safely if you run it in guile mode and drop it before guile mode ends.
    pub unsafe fn new_unchecked() -> Self {
        Self { _marker: () }
    }

    /// # Safety
    ///
    /// You must be in guile mode or never dereference the returned reference.
    pub unsafe fn new_unchecked_ref<'a>() -> &'a Self {
        unsafe { NonNull::<Self>::dangling().as_ref() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_refs() {
        unsafe {
            Guile::new_unchecked_ref();
        }
    }
}
