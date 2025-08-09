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

//! Insert custom types into the guile runtime.

use {
    crate::{
        Guile,
        collections::list::List,
        reference::ReprScm,
        scm::ToScm,
        symbol::Symbol,
        sys::{SCM, scm_unused_struct},
    },
    std::sync::{
        LazyLock,
        atomic::{self, AtomicPtr},
    },
};

/// Return a list comprised of `'(data)`
///
/// This is only exported for the `ForeignObject` derive macro.
///
/// # Safety
///
/// You must be in guile mode.
#[doc(hidden)]
pub unsafe fn slots() -> SCM {
    static SYMBOL: LazyLock<AtomicPtr<scm_unused_struct>> = LazyLock::new(|| {
        let guile = unsafe { Guile::new_unchecked_ref() };
        List::from_iter([Symbol::from_str("data", guile)], guile)
            .to_scm(guile)
            .as_ptr()
            .into()
    });

    SYMBOL.load(atomic::Ordering::Acquire)
}

/// Custom types that can be used in guile.
pub trait ForeignObject: Copy + Send + Sync {
    /// Create a type tag.
    ///
    /// # Safety
    ///
    /// Only call in guile mode.
    unsafe fn get_or_create_type() -> SCM;
}
pub use garguile_proc_macros::ForeignObject;
