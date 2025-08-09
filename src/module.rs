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

//! Manipulate modules and the environment.

use {
    crate::{
        Guile,
        collections::list::List,
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        symbol::Symbol,
        sys::{
            SCM_MODULEP, scm_current_module, scm_defined_p, scm_maybe_resolve_module,
            scm_module_define, scm_module_lookup, scm_module_public_interface, scm_resolve_module,
            scm_variable_ref,
        },
        utils::{c_predicate, scm_predicate},
    },
    std::{borrow::Cow, ffi::CStr},
};

/// Module paths like `'(ice-9 sandbox)`
pub type ModulePath<'gm> = List<'gm, Symbol<'gm>>;

/// Environment containing symbols.
#[repr(transparent)]
pub struct Module<'gm>(Scm<'gm>);
impl<'gm> Module<'gm> {
    /// Get the current module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{module::Module, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let module = Module::current(guile);
    /// }).unwrap();
    /// ```
    pub fn current(guile: &'gm Guile) -> Self {
        Self(Scm::from_ptr(unsafe { scm_current_module() }, guile))
    }

    /// Get a module or create it if it doesn't exist.
    ///
    /// # Examples
    /// ```
    /// # use garguile::{list, module::Module, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let _module = Module::get_or_create(&list!(guile, Symbol::from_str("foo", guile), Symbol::from_str("bar", guile)));
    /// }).unwrap();
    /// ```
    pub fn get_or_create(path: &ModulePath<'gm>) -> Self {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Self::try_from_scm(
            Scm::from_ptr(unsafe { scm_resolve_module(path.as_ptr()) }, guile),
            guile,
        )
        .expect("`scm_resolve_module` should always return a module")
    }

    /// Resolve a module, returning [None] if it does not exist.
    ///
    /// # examples
    ///
    /// ```
    /// # use garguile::{list, module::Module, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(Module::resolve(&list!(guile, Symbol::from_str("ice-9", guile))).is_some());
    /// }).unwrap();
    /// ```
    pub fn resolve(path: &ModulePath<'gm>) -> Option<Self> {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Module::try_from_scm(
            Scm::from_ptr(unsafe { scm_maybe_resolve_module(path.as_ptr()) }, guile),
            guile,
        )
        .ok()
    }

    /// Check if a symbol has been defined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{with_guile, list, module::Module, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(Module::current(guile).is_defined(Symbol::from_str("car", guile)));
    ///     assert!(Module::resolve(&list!(guile, Symbol::from_str("scheme", guile), Symbol::from_str("load", guile))).unwrap().is_defined(Symbol::from_str("load", guile)));
    /// }).unwrap();
    /// ```
    pub fn is_defined(&'gm self, symbol: Symbol<'gm>) -> bool {
        scm_predicate(unsafe { scm_defined_p(symbol.as_ptr(), self.0.as_ptr()) })
    }

    /// Define a symbol in a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, with_guile, module::Module, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let sym = Symbol::from_str("baz", guile);
    ///     let mut module = Module::get_or_create(&list!(guile, Symbol::from_str("foo", guile), Symbol::from_str("bar", guile)));
    ///     module.define(sym, 10);
    ///     assert_eq!(module.read::<i32>(sym).unwrap().unwrap().copied(), 10);
    /// }).unwrap();
    /// ```
    pub fn define<'a, T>(&mut self, sym: Symbol<'gm>, val: T) -> RefMut<'a, 'gm, T>
    where
        T: ToScm<'gm>,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let val = val.to_scm(guile);

        unsafe {
            // SAFETY: we are in guile mode
            scm_module_define(self.0.as_ptr(), sym.as_ptr(), val.as_ptr());

            RefMut::new_unchecked(val.as_ptr())
        }
    }

    /// Read a symbol from a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, with_guile, module::Module, subr::Proc, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let module = Module::resolve(&list!(guile, Symbol::from_str("ice-9", guile), Symbol::from_str("eval-string", guile))).unwrap();
    ///     module.read::<Proc>(Symbol::from_str("eval-string", guile)).unwrap().unwrap();
    ///     assert!(module.read::<Proc>(Symbol::from_str("non-existant-function", guile)).is_none());
    ///     assert!(module.read::<Symbol>(Symbol::from_str("eval-string", guile)).unwrap().is_err());
    /// }).unwrap();
    /// ```
    pub fn read<'module, T>(
        &'gm self,
        sym: Symbol<'gm>,
    ) -> Option<Result<Ref<'module, 'gm, T>, Ref<'module, 'gm, Scm<'gm>>>>
    where
        T: TryFromScm<'gm>,
    {
        self.is_defined(sym).then(|| {
            let guile = unsafe { Guile::new_unchecked_ref() };
            let scm = Scm::from_ptr(
                unsafe { scm_variable_ref(scm_module_lookup(self.0.as_ptr(), sym.as_ptr())) },
                guile,
            );
            unsafe { Ref::from_ptr(scm.as_ptr()) }
        })
    }

    /// Get the public interface of module if it exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, with_guile, module::Module, subr::Proc, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(
    ///         Module::get_or_create(&list!(guile, Symbol::from_str("asdfkljasdflksajfs", guile)))
    ///             .public_interface()
    ///             .is_none()
    ///     );
    /// }).unwrap();
    /// ```
    pub fn public_interface(&self) -> Option<Self> {
        let guile = unsafe { Guile::new_unchecked_ref() };

        Self::try_from_scm(
            Scm::from_ptr(
                unsafe { scm_module_public_interface(self.0.as_ptr()) },
                guile,
            ),
            guile,
        )
        .ok()
    }
}
unsafe impl ReprScm for Module<'_> {}
impl<'gm> TryFromScm<'gm> for Module<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"module")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(unsafe { SCM_MODULEP(scm.as_ptr()) })
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self(scm)
    }
}
impl<'gm> ToScm<'gm> for Module<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.0
    }
}
