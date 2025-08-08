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

//! Utilities for handling lists of procedures.

use {
    crate::{
        Guile,
        collections::list::List,
        reference::ReprScm,
        scm::{Scm, ToScm, TryFromScm},
        subr::{Proc, TupleExt},
        sys::{
            SCM_BOOL_F, SCM_HOOK_ARITY, SCM_HOOKP, scm_add_hook_x, scm_c_run_hook,
            scm_hook_empty_p, scm_make_hook, scm_reset_hook_x,
        },
        utils::{c_predicate, scm_predicate},
    },
    std::{borrow::Cow, ffi::CStr},
};

/// Procedure lists.
///
/// # Examples
///
/// ```
/// # use gargoyle::{hook::Hook, module::Module, symbol::Symbol, with_guile};
/// # fn run_user_config() {}
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///     let init_hook = Hook::<0>::new(guile);
///     let mut module = Module::current(guile);
///     let init_hook = module.define(Symbol::from_str("init-hook", guile), init_hook);
///
///     run_user_config();
///
///     init_hook.run(());
/// }).unwrap();
/// ```
#[repr(transparent)]
pub struct Hook<'gm, const ARITY: usize>(Scm<'gm>);
impl<'gm, const ARITY: usize> Hook<'gm, ARITY> {
    /// Create a new hook.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{hook::Hook, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let hook = Hook::<0>::new(guile);
    /// }).unwrap();
    /// ```
    pub fn new(guile: &'gm Guile) -> Self {
        Self(Scm::from_ptr(
            unsafe { scm_make_hook(ARITY.to_scm(guile).as_ptr()) },
            guile,
        ))
    }

    /// Check if a hook is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{hook::Hook, subr::{guile_fn, GuileFn}, with_guile};
    /// #[guile_fn]
    /// fn foo() {}
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut hook = Hook::<0>::new(guile);
    ///     assert!(hook.is_empty());
    ///     hook.push(Foo::create(guile));
    ///     assert!(!hook.is_empty());
    /// }).unwrap();
    /// ```
    pub fn is_empty(&self) -> bool {
        scm_predicate(unsafe { scm_hook_empty_p(self.as_ptr()) })
    }

    /// Clear all procedures.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{hook::Hook, subr::{guile_fn, GuileFn}, with_guile};
    /// #[guile_fn]
    /// fn foo() {}
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut hook = Hook::<0>::new(guile);
    ///     hook.push(Foo::create(guile));
    ///     assert!(!hook.is_empty());
    ///     hook.clear();
    ///     assert!(hook.is_empty());
    /// }).unwrap();
    /// ```
    pub fn clear(&mut self) {
        unsafe {
            scm_reset_hook_x(self.as_ptr());
        }
    }

    /// Add a procedures to the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{hook::Hook, subr::{guile_fn, GuileFn}, with_guile};
    /// #[guile_fn]
    /// fn foo() {}
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut hook = Hook::<0>::new(guile);
    ///     hook.push(Foo::create(guile));
    ///     assert!(!hook.is_empty());
    /// }).unwrap();
    /// ```
    pub fn push(&mut self, proc: Proc<'gm>) {
        unsafe {
            let guile = Guile::new_unchecked_ref();
            scm_add_hook_x(self.0.as_ptr(), proc.to_scm(guile).as_ptr(), SCM_BOOL_F);
        }
    }

    /// Execute the procedures.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{hook::Hook, subr::{guile_fn, GuileFn}, with_guile};
    /// # use std::sync::atomic::{self, AtomicBool};
    /// # #[cfg(not(miri))] {
    /// static HOOK_RAN: AtomicBool = AtomicBool::new(false);
    /// #[guile_fn]
    /// fn proc() {
    ///     HOOK_RAN.store(true, atomic::Ordering::Release);
    /// }
    /// with_guile(|guile| {
    ///     let mut hook = Hook::new(guile);
    ///     hook.push(Proc::create(guile));
    ///     hook.run(());
    /// }).unwrap();
    /// assert!(HOOK_RAN.load(atomic::Ordering::Acquire));
    /// # }
    /// ```
    pub fn run<T>(&self, args: T)
    where
        T: TupleExt<'gm, ARITY>,
    {
        unsafe {
            // SAFETY: having [self] is proof of being in guile mode
            let guile = Guile::new_unchecked_ref();
            // SAFETY: args must have the same length as the hook arity and this cannot be constructed called without being a hook
            scm_c_run_hook(
                self.0.as_ptr(),
                List::from_iter(args.to_slice(guile).into_iter().rev(), guile)
                    .to_scm(guile)
                    .as_ptr(),
            );
        }
    }
}
unsafe impl<'gm, const ARITY: usize> ReprScm for Hook<'gm, ARITY> {}
impl<'gm, const ARITY: usize> ToScm<'gm> for Hook<'gm, ARITY> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.0
    }
}

impl<'gm, const ARITY: usize> TryFromScm<'gm> for Hook<'gm, ARITY> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"hook")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(unsafe { SCM_HOOKP(scm.as_ptr()) })
            && usize::try_from(unsafe { SCM_HOOK_ARITY(scm.as_ptr()) })
                .map(|arity| arity == ARITY)
                .unwrap_or_default()
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self(scm)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            subr::{GuileFn, guile_fn},
            with_guile,
        },
        std::sync::atomic::{self, AtomicBool},
    };

    #[cfg_attr(miri, ignore)]
    #[test]
    fn hook_is_empty() {
        #[guile_fn(gargoyle_root = crate)]
        fn noop() {}

        static CALLED: AtomicBool = AtomicBool::new(false);
        #[guile_fn(gargoyle_root = crate)]
        fn must_call() {
            CALLED.store(true, atomic::Ordering::Release);
        }

        with_guile(|guile| {
            let mut hook = Hook::<0>::new(guile);
            assert!(hook.is_empty());

            hook.push(Noop::create(guile));
            assert!(!hook.is_empty());
            hook.clear();
            assert!(hook.is_empty());

            hook.push(MustCall::create(guile));
            hook.run(());
            assert!(CALLED.load(atomic::Ordering::Acquire));
        })
        .unwrap();
    }
}
