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

mod alloc;
mod guard;
pub mod sys;

use {
    crate::{alloc::CAllocator, guard::Guard},
    allocator_api2::{alloc::AllocError, vec::Vec as AllocVec},
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
    /// # Safety
    ///
    /// The current thread must be in guile mode.
    pub unsafe fn new_unchecked() -> Self {
        Self { _marker: () }
    }

    /// This will leak memory if you do not [Self::revive] the object.
    pub fn kill<'id, T>(&self, scm: T) -> DeadScm<T::This<'static>>
    where
        T: ScmSubtype<'id>,
    {
        // SAFETY: the `DeadScm` type disables reading
        DeadScm::new(unsafe { scm.cast_lifetime() })
    }

    pub fn revive<'id, T>(&'id self, scm: DeadScm<T>) -> T::This<'id>
    where
        T: ScmSubtype<'static>,
    {
        // SAFETY: we are back in guile mode
        let ptr = unsafe { scm.0.as_ptr() };
        unsafe {
            crate::sys::scm_gc_unprotect_object(ptr);
        }
        unsafe { T::This::<'id>::from_ptr(ptr) }
    }

    pub fn without_guile<F, O>(&mut self, operation: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::<F, O>::eval(operation)
    }

    pub fn make_bool<'id>(&'id self, b: bool) -> Bool<'id> {
        let scm = match b {
            true => unsafe { crate::sys::GARGOYLE_REEXPORTS_SCM_BOOL_T },
            false => unsafe { crate::sys::GARGOYLE_REEXPORTS_SCM_BOOL_F },
        };

        // SAFETY: the scm is a bool
        unsafe { Bool::from_ptr(scm) }
    }
    pub fn make_char<'id>(&'id self, ch: char) -> Char<'id> {
        unsafe {
            Char::from_ptr(sys::scm_integer_to_char(sys::scm_from_uint32(u32::from(
                ch,
            ))))
        }
    }
    pub fn make_string<'id, S>(&'id self, string: &S) -> String<'id>
    where
        S: AsRef<str> + ?Sized,
    {
        let string = string.as_ref();

        let scm =
            unsafe { crate::sys::scm_from_utf8_stringn(string.as_ptr().cast(), string.len()) };
        // SAFETY: this is a string
        unsafe { String::from_ptr(scm) }
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
        operation(&mut unsafe { Api::new_unchecked() })
    }
}
pub fn with_guile<F, O>(operation: F) -> O
where
    F: FnOnce(&mut Api) -> O,
{
    if GUILE_MODE.with(|on| on.load(atomic::Ordering::Acquire)) {
        // SAFETY: we are in guile mode
        operation(&mut unsafe { Api::new_unchecked() })
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

pub trait ScmSubtype<'id>: Sized {
    type This<'a>: ScmSubtype<'a>;

    /// # Safety
    ///
    /// This is safe if you don't smuggle the `SCM` into areas not in guile mode.
    unsafe fn as_ptr(&self) -> crate::sys::SCM;

    /// # Safety
    ///
    /// Make sure the lifetime is accurate.
    unsafe fn from_ptr(_: crate::sys::SCM) -> Self;

    /// # Safety
    ///
    /// This you may cast the lifetime so long as you don't use it to smuggle it to places not in guile mode.
    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b>;
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Any<'id>(RawScm<'id>);
impl<'id> ScmSubtype<'id> for Any<'id> {
    type This<'a> = Any<'a>;

    unsafe fn as_ptr(&self) -> crate::sys::SCM {
        unsafe { self.0.as_ptr() }
    }

    unsafe fn from_ptr(ptr: crate::sys::SCM) -> Self {
        Self(RawScm::from(ptr))
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        Any(unsafe { self.0.cast_lifetime() })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Bool<'id>(RawScm<'id>);
impl<'id> Bool<'id> {
    pub fn to_bool(self) -> bool {
        unsafe { crate::sys::scm_to_bool(self.0.as_ptr()) }.eq(&1)
    }
}
impl<'id> ScmSubtype<'id> for Bool<'id> {
    type This<'a> = Bool<'a>;

    unsafe fn as_ptr(&self) -> crate::sys::SCM {
        unsafe { self.0.as_ptr() }
    }

    unsafe fn from_ptr(ptr: crate::sys::SCM) -> Self {
        Self(RawScm::from(ptr))
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        Bool(unsafe { self.0.cast_lifetime() })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Char<'id>(RawScm<'id>);
impl<'id> Char<'id> {
    pub fn to_char(self) -> char {
        char::from_u32(unsafe { sys::scm_to_uint32(sys::scm_char_to_integer(self.as_ptr())) })
            .unwrap()
    }
}
impl<'id> ScmSubtype<'id> for Char<'id> {
    type This<'a> = Char<'a>;

    unsafe fn as_ptr(&self) -> crate::sys::SCM {
        unsafe { self.0.as_ptr() }
    }

    unsafe fn from_ptr(ptr: crate::sys::SCM) -> Self {
        Self(RawScm::from(ptr))
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        Char(unsafe { self.0.cast_lifetime() })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct String<'id>(RawScm<'id>);
impl<'id> String<'id> {
    pub fn to_string(self) -> string::String<AllocVec<u8, CAllocator>> {
        self.try_to_string().unwrap()
    }

    pub fn try_to_string(self) -> Result<string::String<AllocVec<u8, CAllocator>>, AllocError> {
        let mut len: usize = 0;
        // SAFETY: since we have the lifetime, we can assume we're in guile mode
        let ptr = unsafe { crate::sys::scm_to_utf8_stringn(self.0.as_ptr(), &raw mut len) };
        if ptr.is_null() {
            Err(AllocError)
        } else {
            // SAFETY: we checked for null and since we don't know the capacity we must use length, and the pointer must be freed with [crate::sys::free]
            let vec = unsafe { AllocVec::from_raw_parts_in(ptr.cast(), len, len, CAllocator) };

            // this violates the contract so we should abort.
            assert!(
                str::from_utf8(&vec).is_ok(),
                "The returned string from `scm_to_utf8_stringn` was not utf8. This is bug with guile."
            );

            // SAFETY: we have an assertion above
            Ok(unsafe { string::String::from_utf8_unchecked(vec) })
        }
    }
}
impl<'id> ScmSubtype<'id> for String<'id> {
    type This<'a> = String<'a>;

    unsafe fn as_ptr(&self) -> crate::sys::SCM {
        unsafe { self.0.as_ptr() }
    }

    unsafe fn from_ptr(ptr: crate::sys::SCM) -> Self {
        Self(RawScm::from(ptr))
    }

    unsafe fn cast_lifetime<'b>(self) -> Self::This<'b> {
        String(unsafe { self.0.cast_lifetime() })
    }
}

#[repr(transparent)]
pub struct DeadScm<T>(T)
where
    T: ScmSubtype<'static>;
impl<T> DeadScm<T>
where
    T: ScmSubtype<'static>,
{
    /// Take ownership of the current scm pointer and protect it against garbage collection.
    ///
    /// This will leak memory unless [Api::revive] is called
    fn new(scm: T) -> Self {
        unsafe { crate::sys::scm_gc_protect_object(scm.as_ptr()) };
        Self(scm)
    }
}
/// # Safety
///
/// The pointer is protected.
unsafe impl<T> Send for DeadScm<T> where T: ScmSubtype<'static> {}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Scm<'id> {
    Bool(Bool<'id>),
    Char(Char<'id>),
    String(String<'id>),
    Other(Any<'id>),
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct RawScm<'id> {
    scm: crate::sys::SCM,
    _marker: PhantomData<&'id ()>,
}
impl RawScm<'_> {
    /// # Safety
    /// See [ScmSubtype::as_ptr]
    pub unsafe fn as_ptr(self) -> crate::sys::SCM {
        self.scm
    }

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

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{ops::Deref, thread},
    };

    #[cfg_attr(miri, ignore)]
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

    #[cfg_attr(miri, ignore)]
    #[test]
    fn with_guile_test() {
        assert!(with_guile(|_| true));
        assert!(with_guile(|_| { with_guile(|_| true) },));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn without_guile() {
        assert!(with_guile(|api| {
            api.without_guile(|| with_guile(|_| true))
        }));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn compilation() {
        let tests = trybuild::TestCases::new();
        tests.compile_fail("tests/fail/*.rs");
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn dead_scm_send() {
        with_guile(|api| {
            let t = api.make_bool(true);
            let t = api.kill(t);
            thread::spawn(move || {
                with_guile(move |api| {
                    let _t = api.revive(t);
                });
            });
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn bool_conversion() {
        with_guile(|api| {
            assert_eq!(api.make_bool(true).to_bool(), true);
            assert_eq!(api.make_bool(false).to_bool(), false);
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_conversion() {
        with_guile(|api| {
            assert_eq!(api.make_char('a').to_char(), 'a');
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_conversion() {
        with_guile(|api| {
            assert_eq!(
                api.make_string("hello world").to_string().deref(),
                "hello world"
            );
            assert_eq!(api.make_string("").to_string().deref(), "");
        });
    }
}
