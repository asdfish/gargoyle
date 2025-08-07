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
    crate::{
        Guile,
        collections::list::List,
        reference::Ref,
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

pub type ModulePath<'gm> = List<'gm, Symbol<'gm>>;

#[repr(transparent)]
pub struct Module<'gm>(Scm<'gm>);
impl<'gm> Module<'gm> {
    /// Get the current module.
    pub fn current(guile: &'gm Guile) -> Self {
        Self(Scm::from_ptr(unsafe { scm_current_module() }, guile))
    }

    /// Get a module or create it if it doesn't exist.
    ///
    /// # Examples
    /// ```
    /// # use gargoyle::{list, module::Module, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let _module = Module::get_or_create(&list!(guile, Symbol::from_str("foo", guile), Symbol::from_str("bar", guile)));
    /// }).unwrap();
    /// ```
    pub fn get_or_create(path: &ModulePath<'gm>) -> Self {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Self::try_from_scm(
            Scm::from_ptr(unsafe { scm_resolve_module(path.scm.as_ptr()) }, guile),
            guile,
        )
        .expect("`scm_resolve_module` should always return a module")
    }

    /// Resolve a module, returning [None] if it does not exist.
    ///
    /// # examples
    ///
    /// ```
    /// # use gargoyle::{list, module::Module, symbol::Symbol, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(Module::resolve(&list!(guile, Symbol::from_str("ice-9", guile))).is_some());
    /// }).unwrap();
    /// ```
    pub fn resolve(path: &ModulePath<'gm>) -> Option<Self> {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Module::try_from_scm(
            Scm::from_ptr(
                unsafe { scm_maybe_resolve_module(path.scm.as_ptr()) },
                guile,
            ),
            guile,
        )
        .ok()
    }

    /// Check if a symbol has been defined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{with_guile, list, module::Module, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert!(Module::current(guile).is_defined(Symbol::from_str("car", guile)));
    ///     assert!(Module::resolve(&list!(guile, Symbol::from_str("scheme", guile), Symbol::from_str("load", guile))).unwrap().is_defined(Symbol::from_str("load", guile)));
    /// }).unwrap();
    /// ```
    pub fn is_defined(&'gm self, symbol: Symbol<'gm>) -> bool {
        scm_predicate(unsafe { scm_defined_p(symbol.ptr, self.0.as_ptr()) })
    }

    /// Define a symbol in a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{list, with_guile, module::Module, symbol::Symbol};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let sym = Symbol::from_str("baz", guile);
    ///     let mut module = Module::get_or_create(&list!(guile, Symbol::from_str("foo", guile), Symbol::from_str("bar", guile)));
    ///     module.define(sym, 10);
    ///     assert_eq!(module.read::<i32>(sym).unwrap().unwrap().copied(), 10);
    /// }).unwrap();
    /// ```
    pub fn define<T>(&mut self, sym: Symbol<'gm>, val: T)
    where
        T: ToScm<'gm>,
    {
        unsafe {
            // SAFETY: we are in guile mode
            let guile = Guile::new_unchecked_ref();
            scm_module_define(self.0.as_ptr(), sym.ptr, val.to_scm(guile).as_ptr());
        }
    }

    /// Read a symbol from a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{list, with_guile, module::Module, subr::Proc, symbol::Symbol};
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
                unsafe { scm_variable_ref(scm_module_lookup(self.0.as_ptr(), sym.ptr)) },
                guile,
            );
            unsafe { Ref::from_ptr(scm.ptr) }
        })
    }

    /// Get the public interface of module if it exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{list, with_guile, module::Module, subr::Proc, symbol::Symbol};
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
