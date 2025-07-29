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
mod catch;
pub mod char_set;
mod guard;
pub mod list;
pub mod num;
mod protection;
pub mod string;
pub mod sys;

use {
    crate::{alloc::CAllocator, guard::Guard},
    allocator_api2::{alloc::AllocError, vec::Vec as AllocVec},
    parking_lot::Mutex,
    std::{
        borrow::Cow,
        ffi::{CStr, c_char, c_int, c_void},
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        ops::Not,
        ptr,
        sync::atomic::{self, AtomicBool},
        thread_local,
    },
};

#[doc(inline)]
pub use catch::*;
/// Implement [GuileFn] for a function.
///
/// # Argument types
///
/// ## Optional arguments
///
/// If you want the rest of the arguments to be optional, use the `optional` attribute on the eariliest one.
///
/// The optional argument types must implement [OptionalScm].
///
/// ## Rest arguments
///
/// Rest arguments must be annotated with `#[rest]` and have their type implement [RestScm].
///
/// # Input
///
/// This attribute takes the following arguments.
/// - `guile_ident = "foo"` Change the symbol used in the guile runtime.
/// - `struct_ident = "bar"` Change the identifier used to make a newtype that implements [GuileFn].
///
/// # Examples
///
/// ```
/// # use gargoyle::{GuileFn, Scm};
/// #[gargoyle::guile_fn(guile_ident = "bar", struct_ident = "Baz")]
/// fn foo() {}
/// assert_eq!(Baz::REQUIRED, 0);
/// assert_eq!(Baz::OPTIONAL, 0);
/// assert!(!Baz::REST);
/// assert_eq!(Baz::NAME, c"bar");
/// ```
///
/// ```
/// # use gargoyle::{guile_fn, with_guile};
/// #[guile_fn(guile_ident = "some?")]
/// fn some_p(#[optional] opt: Option<bool>) -> bool {
///     opt.is_some()
/// }
/// # #[cfg(not(miri))]
/// with_guile(|api| {
///     api.define_fn(SomeP);
///     assert_eq!(api.eval_c(c"(some? #f)"), api.make(true));
///     assert_eq!(api.eval_c(c"(some?)"), api.make(false));
/// }).unwrap();
/// ```
pub use proc_macros::guile_fn;
#[doc(inline)]
pub use protection::*;

/// Lock for synchronizing thread initiation.
static INIT_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    /// Whether or not the current thread is in guile mode.
    static GUILE_MODE: AtomicBool = const { AtomicBool::new(false) };
    /// Whether or not the current thread has been initiated yet.
    static THREAD_INIT: AtomicBool = const { AtomicBool::new(false) };
}

struct CallbackData<F, O> {
    operation: Option<F>,
    output: Option<O>,
}

trait GuileModeToggle {
    type Fn;
    type Output;

    /// The status of guile mode in the current thread.
    const GUILE_MODE_STATUS: bool;

    /// This may return [None] in the case of errors.
    fn eval(operation: Self::Fn) -> Option<Self::Output> {
        let mut data = CallbackData {
            operation: Some(operation),
            output: None,
        };

        unsafe {
            crate::sys::scm_with_guile(Some(Self::driver), (&raw mut data).cast());
        }

        data.output
    }

    /// # Safety
    ///
    /// `ptr` must be of type `CallbackData<Self::Fn, Self::Output>`
    unsafe extern "C" fn driver(ptr: *mut c_void) -> *mut c_void {
        GUILE_MODE.with(|on| on.store(Self::GUILE_MODE_STATUS, atomic::Ordering::Release));
        let _guard = Guard::new(|| {
            GUILE_MODE.with(|on| on.store(!Self::GUILE_MODE_STATUS, atomic::Ordering::Release));
        });

        let data = ptr.cast::<CallbackData<Self::Fn, Self::Output>>();
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
    /// This function should be safe to call so long as [GUILE_MODE] is currently [Self::GUILE_MODE_STATUS].
    unsafe fn eval_unchecked(_: Self::Fn) -> Self::Output;
}

/// Struct that gives access to guile functions.
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
    pub fn kill(&self, scm: Scm) -> DeadScm {
        // SAFETY: the `DeadScm` type disables reading
        DeadScm::new(scm)
    }

    /// Revive a [DeadScm] once you're back in guile mode.
    pub fn revive<'id>(&'id self, scm: DeadScm) -> Scm<'id> {
        // SAFETY: we are back in guile mode
        unsafe {
            crate::sys::scm_gc_unprotect_object(scm.0.as_ptr());
        }
        scm.0
    }

    /// Process command line arguments the same way as guile.
    pub fn shell<C, S>(&self, argv: C) -> !
    where
        C: IntoIterator<Item = &'static S>,
        S: AsRef<CStr> + ?Sized + 'static,
    {
        let argv = argv
            .into_iter()
            .map(|arg| arg.as_ref().as_ptr())
            .collect::<Vec<_>>();

        // SAFETY: everything is static
        unsafe { self.shell_raw(argv.leak()) }
    }

    /// [Self::shell] but use raw c style arguments.
    ///
    /// # Safety
    ///
    /// The `*const c_char` pointers must be static and valid to read up to their null character.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// #![no_main]
    /// # use gargoyle::with_guile;
    /// # use std::{ffi::{c_char, c_int}, slice};
    ///
    /// #[unsafe(no_mangle)]
    /// fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    ///     let argv = usize::try_from(argc)
    ///         .ok()
    ///         .filter(|_| !argv.is_null())    
    ///         .map(|argc| unsafe { slice::from_raw_parts(argv, argc) })
    ///         .unwrap_or_default();
    ///     with_guile(|api| {
    ///         unsafe { api.shell_raw(argv); }
    ///     });
    ///     0
    /// }
    /// ```
    pub unsafe fn shell_raw(&self, argv: &'static [*const c_char]) -> ! {
        unsafe {
            sys::scm_shell(argv.len().try_into().unwrap(), argv.as_ptr());
        }

        unreachable!()
    }

    /// Execute a function without access to the guile api.
    ///
    /// If you went a long time without calling a `sys::scm_*` function, the garbage collection would not occur.
    pub fn without_guile<F, O>(&mut self, operation: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::<F, O>::eval(operation)
            .expect("running outside of guile mode should not be able to get guile errors")
    }

    /// Create a [Scm].
    pub fn make<'id, 'b, T>(&'id self, with: T) -> Scm<'id>
    where
        T: ScmTy<'b>,
    {
        unsafe { T::construct(with).cast_lifetime() }
    }

    /// Create a function but do not create a binding to it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{guile_fn, with_guile};
    /// #[guile_fn]
    /// fn my_hidden_fn() {}
    /// # #[cfg(not(miri))] {
    /// let output = with_guile(|api| {
    ///     let my_hidden_fn = api.make_fn(MyHiddenFn);
    ///     assert_eq!(my_hidden_fn.call(&mut []), api.make(()));
    ///     api.eval_c(c"(my-hidden-fn)");
    /// });
    /// assert_eq!(output, None);
    /// # }
    /// ```
    pub fn make_fn<'id, F>(&'id self, _: F) -> Scm<'id>
    where
        F: GuileFn,
    {
        unsafe {
            Scm::from_ptr(sys::scm_c_make_gsubr(
                F::NAME.as_ptr(),
                c_int::try_from(F::REQUIRED).unwrap(),
                c_int::try_from(F::OPTIONAL).unwrap(),
                c_int::from(F::REST),
                F::ADDR,
            ))
        }
    }

    /// Define a function in the guile environment making it accessible to all threads.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{guile_fn, with_guile};
    /// # use std::thread;
    /// #
    /// #[guile_fn]
    /// fn my_not(b: bool) -> bool { !b }
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///     api.define_fn(MyNot);
    ///     thread::spawn(|| {
    ///         with_guile(|api| {
    ///             assert_eq!(api.eval_c(c"(my-not #f)"), api.make(true));
    ///         }).unwrap();
    ///     });
    ///     assert_eq!(api.eval_c(c"(my-not #t)"), api.make(false));
    /// }).unwrap();
    /// ```
    pub fn define_fn<'id, F>(&'id self, _: F) -> Scm<'id>
    where
        F: GuileFn,
    {
        unsafe {
            Scm::from_ptr(sys::scm_c_define_gsubr(
                F::NAME.as_ptr(),
                c_int::try_from(F::REQUIRED).unwrap(),
                c_int::try_from(F::OPTIONAL).unwrap(),
                c_int::from(F::REST),
                F::ADDR,
            ))
        }
    }

    /// Evaluate an expression and return its result.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::with_guile;
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///    assert_eq!(api.eval_c(c"#t"), api.make(true));
    /// }).unwrap();
    /// ```
    pub fn eval_c<'id, S>(&'id self, expr: &S) -> Scm<'id>
    where
        S: AsRef<CStr> + ?Sized,
    {
        unsafe { Scm::from_ptr(sys::scm_c_eval_string(expr.as_ref().as_ptr())) }
    }

    /// Evaluate the contents of a file.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{guile_fn, with_guile};
    /// # use std::{io::Write, ffi::CString};
    /// # use tempfile::NamedTempFile;
    /// #[guile_fn]
    /// fn return_true() -> bool { true }
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///     api.define_fn(ReturnTrue);
    ///     let mut file = NamedTempFile::new().unwrap();
    ///     write!(file, "(return-true)").unwrap();
    ///     let output = api.load_c(&CString::new(file.path().as_os_str().as_encoded_bytes()).unwrap());
    ///     assert_eq!(output, api.make(true));
    /// }).unwrap();
    /// ```
    pub fn load_c<'id, S>(&'id self, expr: &S) -> Scm<'id>
    where
        S: AsRef<CStr> + ?Sized,
    {
        unsafe { Scm::from_ptr(sys::scm_c_primitive_load(expr.as_ref().as_ptr())) }
    }

    /// Throw an error for having the wrong type.
    pub fn wrong_type_arg<F, E>(&self, name: &F, idx: usize, arg: Scm, expected: &E) -> !
    where
        F: AsRef<CStr> + ?Sized,
        E: AsRef<CStr> + ?Sized,
    {
        unsafe {
            sys::scm_wrong_type_arg_msg(
                name.as_ref().as_ptr(),
                idx.try_into().unwrap(),
                arg.as_ptr(),
                expected.as_ref().as_ptr(),
            );
        }

        unreachable!()
    }

    pub fn misc_error<F, M>(&self, name: &F, msg: &M, args: Scm) -> !
    where
        F: AsRef<CStr> + ?Sized,
        M: AsRef<CStr> + ?Sized,
    {
        unsafe { sys::scm_misc_error(name.as_ref().as_ptr(), msg.as_ref().as_ptr(), args.as_ptr()) }
        unreachable!()
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
/// Execute a function with access to the guile api.
///
/// This may return [None] in the event of errors.
///
/// # Examples
///
/// ```
/// # use gargoyle::{guile_fn, with_guile};
/// #[guile_fn]
/// fn my_sub(l: i32, r: i32) -> i32 { l - r }
/// # #[cfg(not(miri))] {
/// let output = with_guile(|api| {
///     api.define_fn(MySub);
///     api.eval_c(c"(my-sub #f \"bar\")"); // type error
/// });
/// assert_eq!(output, None);
/// let output = with_guile(|api| {
///     api.eval_c(c"(my-sub 3 2)").get::<i32>()
/// });
/// assert_eq!(output, Some(Some(1)));
/// # }
/// ```
pub fn with_guile<F, O>(operation: F) -> Option<O>
where
    F: FnOnce(&mut Api) -> O,
{
    if GUILE_MODE.with(|on| on.load(atomic::Ordering::Acquire)) {
        // SAFETY: we are in guile mode
        Some(operation(&mut unsafe { Api::new_unchecked() }))
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

/// Protect a [Scm] from garbage collection and then make it unreadable.
///
/// To get the [Scm] back, use [Api::revive].
///
/// This is the equivalent of [Weak][std::rc::Weak] for [Rc][std::rc::Rc].
#[repr(transparent)]
pub struct DeadScm(Scm<'static>);
impl DeadScm {
    /// Take ownership of the current scm pointer and protect it against garbage collection.
    ///
    /// This will leak memory unless [Api::revive] is called
    fn new(scm: Scm) -> Self {
        unsafe { crate::sys::scm_gc_protect_object(scm.as_ptr()) };
        Self(unsafe { scm.cast_lifetime() })
    }
}
/// # Safety
///
/// The pointer is protected.
unsafe impl Send for DeadScm {}

/// A newtype for [SCM][sys::SCM] pointers.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Scm<'id> {
    scm: crate::sys::SCM,
    _marker: PhantomData<&'id ()>,
}
impl<'id> Scm<'id> {
    /// # Safety
    ///
    /// This is safe if you don't use it to smuggle this object outside of guile mode.
    pub unsafe fn as_ptr(&self) -> crate::sys::SCM {
        self.scm
    }

    /// # Safety
    ///
    /// This is safe if you don't use it to smuggle this object outside of guile mode.
    pub unsafe fn cast_lifetime<'b>(self) -> Scm<'b> {
        Scm {
            scm: self.scm,
            _marker: PhantomData,
        }
    }

    /// Check if this [Scm] is truthy.
    pub fn is_true(&self) -> bool {
        unsafe { sys::scm_is_true(self.as_ptr()) }
    }

    /// Check whether or not this [Scm] is a `T`
    pub fn is<T>(&self) -> bool
    where
        T: ScmTy<'id>,
    {
        let api = unsafe { Api::new_unchecked() };
        T::predicate(&api, self)
    }
    /// Attempt to get `T` from a scm
    pub fn get<T>(&self) -> Option<T::Output>
    where
        T: ScmTy<'id>,
    {
        let api = unsafe { Api::new_unchecked() };

        if self.is::<T>() {
            Some(unsafe { T::get_unchecked(&api, self) })
        } else {
            None
        }
    }

    /// Check equality with `eq?` semantics
    pub fn is_eq(&self, r: &Self) -> bool {
        unsafe { Self::from_ptr(sys::scm_eq_p(self.as_ptr(), r.as_ptr())) }.is_true()
    }

    /// Check equality with `eqv?` semantics
    pub fn is_eqv(&self, r: &Self) -> bool {
        unsafe { Self::from_ptr(sys::scm_eqv_p(self.as_ptr(), r.as_ptr())) }.is_true()
    }

    /// Check equality with `equal?` semantics
    pub fn is_equal(&self, r: &Self) -> bool {
        unsafe { Self::from_ptr(sys::scm_equal_p(self.as_ptr(), r.as_ptr())) }.is_true()
    }

    /// Call a function
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::{guile_fn, with_guile};
    /// #[guile_fn]
    /// fn my_mul(l: i32, r: i32) -> i32 { l * r }
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///     let my_mul = api.define_fn(MyMul);
    ///     assert_eq!(my_mul.call(&mut [api.make(4), api.make(2)]), api.make(8));
    /// }).unwrap();
    /// ```
    pub fn call<T>(&self, args: &mut T) -> Self
    where
        T: AsMut<[Self]>,
    {
        let args = args.as_mut();

        unsafe {
            Scm::from_ptr(sys::scm_call_n(
                self.as_ptr(),
                // SAFETY: Scm is `repr(transparent)` to a [SCM].
                args.as_mut_ptr().cast(),
                args.len(),
            ))
        }
    }

    /// # Safety
    ///
    /// The lifetime should be associated with an [Api] object.
    pub unsafe fn from_ptr(scm: crate::sys::SCM) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}
impl Display for Scm<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        unsafe {
            let port = sys::scm_open_output_string();
            sys::scm_write(self.as_ptr(), port);
            let text = sys::scm_strport_to_string(port);
            sys::scm_close_port(port);
            Scm::from_ptr(text)
        }
        .get::<&str>()
        .ok_or(fmt::Error)
        .and_then(|result| result.map_err(|_| fmt::Error))
        .and_then(|display| display.fmt(f))
    }
}
impl PartialEq for Scm<'_> {
    /// See [Self::is_equal].
    fn eq(&self, r: &Self) -> bool {
        self.is_equal(r)
    }
}
impl Not for Scm<'_> {
    type Output = Option<Self>;

    fn not(self) -> Option<Self> {
        if self.is::<bool>() {
            Some(unsafe { Self::from_ptr(sys::scm_not(self.as_ptr())) })
        } else {
            None
        }
    }
}

/// Marker trait for types that can be converted to/from a [Scm].
pub trait ScmTy<'id>: Sized {
    /// The output of [Self::get_unchecked]. If unsure, you should default to `Self`.
    type Output;

    fn type_name() -> Cow<'static, CStr>;

    /// Create a [Scm] from the current type.
    fn construct(self) -> Scm<'id>;
    /// Check whether or not a [Scm] is of this type.
    fn predicate(_: &Api, _: &Scm) -> bool;
    /// Exract [Self::Output] from a scm.
    ///
    /// # Safety
    ///
    /// This function must be safe if [Self::predicate] returns [true].
    unsafe fn get_unchecked(_: &Api, _: &Scm) -> Self::Output;
}
impl<'id> ScmTy<'id> for () {
    type Output = ();

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"<#undefined>")
    }

    fn construct(self) -> Scm<'id> {
        unsafe { Scm::from_ptr(sys::SCM_UNDEFINED) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { crate::sys::SCM_UNBNDP(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, _: &Scm) -> Self {}
}
impl<'id> ScmTy<'id> for bool {
    type Output = Self;

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"bool")
    }

    fn construct(self) -> Scm<'id> {
        let scm = match self {
            true => unsafe { crate::sys::SCM_BOOL_T },
            false => unsafe { crate::sys::SCM_BOOL_F },
        };

        unsafe { Scm::from_ptr(scm) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { crate::sys::scm_is_bool(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self {
        unsafe { crate::sys::scm_to_bool(scm.as_ptr()) }
    }
}
impl<'id> ScmTy<'id> for char {
    type Output = Self;

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"char")
    }

    fn construct(self) -> Scm<'id> {
        unsafe {
            Scm::from_ptr(sys::scm_integer_to_char(sys::scm_from_uint32(u32::from(
                self,
            ))))
        }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::gargoyle_reexports_scm_is_true(sys::scm_char_p(scm.as_ptr())) }
    }

    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> char {
        char::from_u32(unsafe { sys::scm_to_uint32(sys::scm_char_to_integer(scm.as_ptr())) })
            .expect("Guile characters should return valid rust characters.")
    }
}
impl<'id> ScmTy<'id> for &str {
    type Output = Result<::string::String<AllocVec<u8, CAllocator>>, AllocError>;

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"string")
    }

    fn construct(self) -> Scm<'id> {
        let scm = unsafe { crate::sys::scm_from_utf8_stringn(self.as_ptr().cast(), self.len()) };
        unsafe { Scm::from_ptr(scm) }
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_string(scm.as_ptr()) }
    }

    unsafe fn get_unchecked(
        _: &Api,
        scm: &Scm,
    ) -> Result<::string::String<AllocVec<u8, CAllocator>>, AllocError> {
        let mut len: usize = 0;
        // SAFETY: since we have the lifetime, we can assume we're in guile mode
        let ptr = unsafe { crate::sys::scm_to_utf8_stringn(scm.as_ptr(), &raw mut len) };
        if ptr.is_null() {
            Err(AllocError)
        } else {
            // SAFETY: we checked for null and since we don't know the capacity we must use length, and the pointer must be freed with [crate::sys::free]
            let vec = unsafe { AllocVec::from_raw_parts_in(ptr.cast(), len, len, CAllocator) };

            // this violates the contract so we should abort.
            assert!(
                str::from_utf8(&vec).is_ok(),
                "The returned string from `scm_to_utf8_stringn` was not utf8. This is a bug with guile."
            );

            // SAFETY: we have an assertion above
            Ok(unsafe { ::string::String::from_utf8_unchecked(vec) })
        }
    }
}
impl<'id> ScmTy<'id> for Scm<'id> {
    type Output = Self;

    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"scm")
    }

    fn construct(self) -> Scm<'id> {
        unsafe { Scm::from_ptr(self.as_ptr()) }
    }
    fn predicate(_: &Api, _: &Scm) -> bool {
        true
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        unsafe { Scm::from_ptr(scm.as_ptr()) }
    }
}

/// Marker trait for types that can be used with the `#[optional]` attribute in [guile_fn]
pub trait OptionalScm<'id>
where
    Self: From<Option<Self::Inner>> + Into<Option<Self::Inner>>,
{
    type Inner: ScmTy<'id>;
}
impl<'id, T> OptionalScm<'id> for Option<T>
where
    T: ScmTy<'id>,
{
    type Inner = T;
}

/// Marker trait for types that can be used with the `#[rest]` attribute in [guile_fn]
pub trait RestScm<'a>: From<Scm<'a>> {}
impl<'a> RestScm<'a> for Scm<'a> {}

/// Trait for describing functions that can be added into the runtime with [Api::define_fn].
pub trait GuileFn {
    /// The function pointer to an `extern "C"` function with an arity of `Self::REQUIRED + Self::OPTIONAL + Self::REST` that takes [sys::SCM]s
    const ADDR: *mut c_void;
    /// The name of this function in guile.
    const NAME: &CStr;

    /// The amount of required arguments.
    const REQUIRED: usize;
    /// The amount of `&optional` arguments.
    const OPTIONAL: usize;
    /// Whether or not there are `&rest` arguments.
    const REST: bool;

    /// Assert that [Self::REQUIRED] and [Self::OPTIONAL] are less than 10.
    const _ARITY_CHECK: () = {
        assert!(Self::REQUIRED + Self::OPTIONAL + Self::REST as usize <= 10);
    };
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{
            fmt::Debug,
            io::{self, Write},
            thread,
        },
    };

    #[cfg_attr(miri, ignore)]
    #[test]
    fn compilation() {
        let tests = trybuild::TestCases::new();
        tests.compile_fail("tests/fail/*.rs");
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn multi_threading() {
        let spawn = || thread::spawn(|| with_guile(|_| {}));
        [(); 2]
            .map(|_| spawn())
            .into_iter()
            .map(|thread| thread.join())
            .collect::<Result<Option<Vec<_>>, _>>()
            .unwrap()
            .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn with_guile_test() {
        assert_eq!(with_guile(|_| true), Some(true));
        assert_eq!(with_guile(|_| { with_guile(|_| true) }), Some(Some(true)));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn without_guile() {
        assert_eq!(
            with_guile(|api| { api.without_guile(|| with_guile(|_| true)) }),
            Some(Some(true))
        );
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn dead_scm_send() {
        with_guile(|api| {
            let t = api.make(true);
            let t = api.kill(t);
            thread::spawn(move || {
                with_guile(move |api| {
                    let t = api.revive(t);
                    assert_eq!(t.get::<bool>(), Some(true));
                })
                .unwrap();
            });
        });
    }

    pub trait ApiExt {
        fn test_real<'id, T>(&'id self, _: T, _: T::Output) -> Scm<'id>
        where
            T: ScmTy<'id>,
            T::Output: Debug + PartialEq;

        fn test_real_equal<'id, T>(&'id self, val: T) -> Scm<'id>
        where
            T: Clone + Debug + PartialEq + ScmTy<'id, Output = T>,
        {
            let scm = self.test_real(val.clone(), val);
            assert!(scm.is_eqv(&scm));
            scm
        }
    }
    impl ApiExt for Api {
        fn test_real<'id, T>(&'id self, val: T, output: T::Output) -> Scm<'id>
        where
            T: ScmTy<'id>,
            T::Output: Debug + PartialEq,
        {
            let scm = self.make(val);
            assert!(T::predicate(self, &scm));
            assert!(scm.eq(&scm));
            assert_eq!(unsafe { T::get_unchecked(self, &scm) }, output);

            scm
        }
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn bool_conversion() {
        with_guile(|api| {
            api.test_real_equal(true);
            api.test_real_equal(false);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_conversion() {
        with_guile(|api| {
            api.test_real_equal(char::MIN);
            api.test_real_equal(char::MAX);
            ('a'..='z')
                .into_iter()
                .map(|ch| api.test_real_equal(ch))
                .for_each(drop);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_conversion() {
        with_guile(|api| {
            let mut hello_world = AllocVec::new_in(CAllocator);
            hello_world.extend(b"hello world");
            api.test_real(
                "hello world",
                Ok(unsafe { ::string::String::from_utf8_unchecked(hello_world) }),
            );

            let empty = AllocVec::new_in(CAllocator);
            api.test_real(
                "",
                Ok(unsafe { ::string::String::from_utf8_unchecked(empty) }),
            );
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn display_test() {
        with_guile(|api| {
            // display implementation by guile may change in the future so we can only assert success
            assert!(write!(io::empty(), "{}", api.make(true)).is_ok());
        })
        .unwrap();
    }
}
