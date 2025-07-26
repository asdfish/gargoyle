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

//! Rust bindings to guile.

use {
    crate::guard::Guard,
    parking_lot::Mutex,
    std::{
        ffi::c_void,
        marker::PhantomData,
        ptr,
        sync::atomic::{self, AtomicBool},
        thread_local,
    },
};

/// Lock for synchronizing thread initiation.
static INIT_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    /// Whether or not the current thread is in guile mode.
    static GUILE_MODE: AtomicBool = const { AtomicBool::new(false) };
    /// Whether or not the current thread has been initiated yet.
    static THREAD_INIT: AtomicBool = const { AtomicBool::new(false) };
}

struct GuileModeToggleCallbackData<F, O> {
    operation: Option<F>,
    output: Option<O>,
}

trait GuileModeToggle {
    type Fn;
    type Output;

    /// The status of guile mode in the current thread.
    const GUILE_MODE_STATUS: bool;

    fn eval(operation: Self::Fn) -> Self::Output {
        let mut data = GuileModeToggleCallbackData {
            operation: Some(operation),
            output: None,
        };

        unsafe {
            crate::sys::scm_with_guile(Some(Self::driver), (&raw mut data).cast());
        }

        data.output
            .expect("`Self::driver` should be called by `scm_with_guile`")
    }

    /// # Safety
    ///
    /// `ptr` must be of type `GuileModeToggleCallbackData<Self::Fn, Self::Output>`
    unsafe extern "C" fn driver(ptr: *mut c_void) -> *mut c_void {
        GUILE_MODE.with(|on| on.store(Self::GUILE_MODE_STATUS, atomic::Ordering::Release));
        let _guard = Guard::new(|| {
            GUILE_MODE.with(|on| on.store(!Self::GUILE_MODE_STATUS, atomic::Ordering::Release));
        });

        let data = ptr.cast::<GuileModeToggleCallbackData<Self::Fn, Self::Output>>();
        if let Some(data) = unsafe { data.as_mut() } {
            if data.output.is_none() {
                data.output = data
                    .operation
                    .take()
                    .map(|operation| unsafe { Self::eval_unchecked(operation) });
            }
        }

        ptr::null_mut()
    }

    /// # Safety
    ///
    /// This function should be safe to call so long as [GUILE_MODE] is currently [Self::STATUS]
    unsafe fn eval_unchecked(_: Self::Fn) -> Self::Output;
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Api {
    _marker: (),
}
impl Api {
    fn new() -> Self {
        Self { _marker: () }
    }
}

struct WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    _marker: PhantomData<F>,
}
impl<F, O> GuileModeToggle for WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    type Fn = F;
    type Output = O;

    const GUILE_MODE_STATUS: bool = false;

    unsafe fn eval_unchecked(operation: Self::Fn) -> Self::Output {
        operation()
    }
}
impl Api {
    pub fn revive<'id, T>(&'id self, scm: DeadScm<T>) -> T::This<'id>
    where
        T: ScmSubtype<'static>,
    {
        // SAFETY: we are back in guile mode
        unsafe { scm.0.cast_lifetime() }
    }

    pub fn without_guile<F, O>(&mut self, operation: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::<F, O>::eval(operation)
    }
}

struct WithGuile<F, O>
where
    F: FnOnce(&mut Api) -> O,
{
    _marker: PhantomData<(F, O)>,
}
impl<F, O> GuileModeToggle for WithGuile<F, O>
where
    F: FnOnce(&mut Api) -> O,
{
    type Fn = F;
    type Output = O;

    const GUILE_MODE_STATUS: bool = true;

    unsafe fn eval_unchecked(operation: Self::Fn) -> Self::Output {
        operation(&mut Api::new())
    }
}
pub fn with_guile<F, O>(operation: F) -> O
where
    F: FnOnce(&mut Api) -> O,
{
    if GUILE_MODE.with(|on| on.load(atomic::Ordering::Acquire)) {
        operation(&mut Api::new())
    } else {
        let _lock = THREAD_INIT
            .with(|init| !init.load(atomic::Ordering::Acquire))
            .then(|| INIT_LOCK.lock());

        WithGuile::eval(|api| {
            THREAD_INIT.with(|init| init.store(true, atomic::Ordering::Release));

            operation(api)
        })
    }
}

impl Api {
    pub fn make_bool<'id>(&'id self, b: bool) -> Bool<'id> {
        Bool::new(RawScm::from(match b {
            true => unsafe { crate::sys::G_REEXPORTS_SCM_BOOL_T },
            false => unsafe { crate::sys::G_REEXPORTS_SCM_BOOL_T },
        }))
    }
}

pub trait ScmSubtype<'id>: Sized {
    type This<'a>: ScmSubtype<'a>;

    fn from_raw(_: RawScm<'id>) -> Self;
    fn into_scm(self) -> Scm<'id>;

    /// # Safety
    ///
    /// This you may cast the lifetime so long as you don't use it to smuggle it to places not in guile mode.
    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b>;
    fn kill(self) -> DeadScm<Self::This<'static>> {
        // SAFETY: the `DeadScm` type disables reading
        DeadScm(unsafe { self.cast_lifetime() })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Any<'id>(RawScm<'id>);
impl<'id> ScmSubtype<'id> for Any<'id> {
    type This<'a> = Any<'a>;

    fn from_raw(scm: RawScm<'id>) -> Self {
        Self(scm)
    }
    fn into_scm(self) -> Scm<'id> {
        Scm::Other(self)
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        Any(unsafe { self.0.cast_lifetime() })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Bool<'id>(RawScm<'id>);
impl<'id> Bool<'id> {
    fn new(scm: RawScm<'id>) -> Self {
        Self(scm)
    }
}
impl<'id> ScmSubtype<'id> for Bool<'id> {
    type This<'a> = Bool<'a>;

    fn from_raw(scm: RawScm<'id>) -> Self {
        Self(scm)
    }
    fn into_scm(self) -> Scm<'id> {
        Scm::Bool(self)
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        Bool(unsafe { self.0.cast_lifetime() })
    }
}

#[repr(transparent)]
pub struct DeadScm<T>(T)
where
    T: ScmSubtype<'static>;

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Scm<'id> {
    Bool(Bool<'id>),
    Other(Any<'id>),
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct RawScm<'id> {
    scm: crate::sys::SCM,
    _marker: PhantomData<&'id ()>,
}
impl RawScm<'_> {
    /// # Safety
    ///
    /// See [ScmSubtype::cast_lifetime]
    pub unsafe fn cast_lifetime<'b>(self) -> RawScm<'b> {
        RawScm {
            scm: self.scm,
            _marker: PhantomData,
        }
    }
}
impl From<crate::sys::SCM> for RawScm<'_> {
    fn from(scm: crate::sys::SCM) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

mod guard;

pub mod sys {
    //! Low level bindings to guile.
    //!
    //! Only the `scm*` symbols can be guaranteed to exist.

    #![allow(improper_ctypes)]
    #![expect(non_camel_case_types)]
    #![expect(non_snake_case)]
    #![expect(non_upper_case_globals)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(test)]
mod tests {
    use {super::*, std::thread};

    #[test]
    fn multi_threading() {
        let spawn = || thread::spawn(|| with_guile(|_| {}));
        [(); 2]
            .map(|_| spawn())
            .into_iter()
            .map(|thread| thread.join())
            .collect::<Result<(), _>>()
            .unwrap();
    }

    #[test]
    fn with_guile_test() {
        assert!(with_guile(|_| true));
        assert!(with_guile(|_| { with_guile(|_| true) },));
    }

    #[test]
    fn without_guile() {
        assert!(with_guile(|api| {
            api.without_guile(|| with_guile(|_| true))
        }));
    }

    #[test]
    fn compilation() {
        let tests = trybuild::TestCases::new();
        tests.compile_fail("tests/fail/*.rs");
    }
}
