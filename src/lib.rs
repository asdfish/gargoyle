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
        cmp::Ordering,
        ffi::{CStr, c_double, c_int, c_void},
        marker::PhantomData,
        ops::Not,
        ptr,
        sync::atomic::{self, AtomicBool},
        thread_local,
    },
};

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
/// ```
/// #[gargoyle::guile_fn]
/// fn foo(#[optional] l: Option<bool>, r: Option<bool>) -> bool {
///     println!("{} {}", l.is_some(), r.is_some());
///     true
/// }
/// // (foo 1 2) -> "true true"
/// // (foo 1) -> "true false"
/// // (foo) -> "false false"
/// ```
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
/// #[gargoyle::guile_fn(guile_ident = "bar")]
/// fn foo(#[optional] _: Option<bool>, _: Option<i32>, #[rest] _rest: Scm) -> bool { true }
/// assert_eq!(Foo::REQUIRED, 0);
/// assert_eq!(Foo::OPTIONAL, 2);
/// assert!(Foo::REST);
/// assert_eq!(Foo::NAME, c"bar");
/// ```
pub use proc_macros::guile_fn;

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

        data.output.expect(
            "`Self::driver` should be called by `scm_with_guile` which populates `data.output`",
        )
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
    pub fn kill(&self, scm: Scm) -> DeadScm {
        // SAFETY: the `DeadScm` type disables reading
        DeadScm::new(scm)
    }

    pub fn revive<'id>(&'id self, scm: DeadScm) -> Scm<'id> {
        // SAFETY: we are back in guile mode
        unsafe {
            crate::sys::scm_gc_unprotect_object(scm.0.as_ptr());
        }
        scm.0
    }

    pub fn without_guile<F, O>(&mut self, operation: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::<F, O>::eval(operation)
    }

    pub fn make<'id, T>(&'id self, with: T) -> Scm<'id>
    where
        T: ScmTy,
    {
        T::construct(with, self)
    }

    /// # Panics
    ///
    /// This function will panic if [GuileFn::REQUIRED] and [GuileFn::OPTIONAL] are not convertible into a [c_int] but that should not be possible unless you overwrote the [GuileFn::_LENGTH_CHECK] field.
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
    /// # use gargoyle::{guile_fn, with_guile};
    /// #[guile_fn]
    /// fn return_true() -> bool { true }
    /// with_guile(|api| {
    ///    api.define_fn(ReturnTrue);
    ///    assert_eq!(api.eval(c"(return-true)"), api.make(true));
    /// })
    /// ```
    pub fn eval<'id, S>(&'id self, expr: &S) -> Scm<'id>
    where
        S: AsRef<CStr> + ?Sized,
    {
        unsafe { Scm::from_ptr(sys::scm_c_eval_string(expr.as_ref().as_ptr())) }
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

#[derive(Debug)]
#[repr(transparent)]
pub struct Scm<'id> {
    scm: crate::sys::SCM,
    _marker: PhantomData<&'id ()>,
}
impl Scm<'_> {
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

    pub fn is_true(&self) -> bool {
        unsafe { sys::scm_is_true(self.as_ptr()) }
    }

    pub fn is<T>(&self) -> bool
    where
        T: ScmTy,
    {
        let api = unsafe { Api::new_unchecked() };
        T::predicate(&api, self)
    }
    /// Attempt to get `T` from a scm
    pub fn get<T>(&self) -> Option<T::Output>
    where
        T: ScmTy,
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

    /// Check whether or not the [Scm] is a number.
    pub fn is_number(&self) -> bool {
        unsafe { sys::scm_is_number(self.as_ptr()) }
    }
    /// Check whether or not the [Scm] is a real number.
    pub fn is_real_number(&self) -> bool {
        unsafe { sys::scm_is_real(self.as_ptr()) }
    }
}
impl PartialOrd for Scm<'_> {
    fn partial_cmp(&self, r: &Self) -> Option<Ordering> {
        [
            (
                sys::scm_less_p as unsafe extern "C" fn(_: sys::SCM, _: sys::SCM) -> sys::SCM,
                Ordering::Less,
            ),
            (sys::scm_num_eq_p, Ordering::Equal),
            (sys::scm_gr_p, Ordering::Greater),
        ]
        .into_iter()
        .find_map(|(predicate, output)| {
            unsafe { Self::from_ptr((predicate)(self.as_ptr(), r.as_ptr())) }
                .is_true()
                .then_some(output)
        })
    }
}
impl Scm<'_> {
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

pub trait ScmTy: Sized {
    type Output;

    /// Create a [Scm] from the current type.
    fn construct<'id>(self, _: &'id Api) -> Scm<'id>;
    /// Check whether or not a [Scm] is of this type.
    fn predicate(_: &Api, _: &Scm) -> bool;
    /// Exract [Self::Output] from a scm.
    ///
    /// # Safety
    ///
    /// This function must be safe if [Self::predicate] returns [true].
    unsafe fn get_unchecked(_: &Api, _: &Scm) -> Self::Output;
}
impl ScmTy for () {
    type Output = ();

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { Scm::from_ptr(sys::SCM_UNDEFINED) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { crate::sys::SCM_UNBNDP(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, _: &Scm) -> Self {}
}
impl ScmTy for bool {
    type Output = Self;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
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
impl ScmTy for char {
    type Output = Self;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
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
impl ScmTy for c_double {
    type Output = Self;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        unsafe { Scm::from_ptr(sys::scm_from_double(self)) }
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        scm.is_real_number()
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self {
        unsafe { crate::sys::scm_to_double(scm.as_ptr()) }
    }
}
impl ScmTy for &str {
    type Output = Result<string::String<AllocVec<u8, CAllocator>>, AllocError>;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        let scm = unsafe { crate::sys::scm_from_utf8_stringn(self.as_ptr().cast(), self.len()) };
        unsafe { Scm::from_ptr(scm) }
    }

    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_string(scm.as_ptr()) }
    }

    unsafe fn get_unchecked(
        _: &Api,
        scm: &Scm,
    ) -> Result<string::String<AllocVec<u8, CAllocator>>, AllocError> {
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
                "The returned string from `scm_to_utf8_stringn` was not utf8. This is bug with guile."
            );

            // SAFETY: we have an assertion above
            Ok(unsafe { string::String::from_utf8_unchecked(vec) })
        }
    }
}

macro_rules! impl_scm_ty_for_int {
    ([ $(($ty:ty, $ptr:ty, $predicate:expr, $to_scm:expr, $to_int:expr $(,)?)),+ $(,)? ]) => {
        $(impl_scm_ty_for_int!($ty, $ptr, $predicate, $to_scm, $to_int);)+
    };
    ($ty:ty, $ptr:ty, $predicate:expr, $to_scm:expr, $to_int:expr) => {
        impl ScmTy for $ty {
            type Output = Self;

            fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
                unsafe { Scm::from_ptr(($to_scm)(self)) }
            }
            fn predicate(_: &Api, scm: &Scm) -> bool {
                unsafe {
                    ($predicate)(
                        scm.as_ptr(),
                        <$ty>::MIN as $ptr,
                        <$ty>::MAX as $ptr,
                    )
                }
            }
            unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
                unsafe { ($to_int)(scm.as_ptr()) }
            }
        }
    };
}
impl_scm_ty_for_int!([
    (
        i8,
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int8,
        sys::scm_to_int8
    ),
    (
        i16,
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int16,
        sys::scm_to_int16
    ),
    (
        i32,
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int32,
        sys::scm_to_int32
    ),
    (
        isize,
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_intptr_t,
        sys::scm_to_intptr_t
    ),
    (
        u8,
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint8,
        sys::scm_to_uint8
    ),
    (
        u16,
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint16,
        sys::scm_to_uint16
    ),
    (
        u32,
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint32,
        sys::scm_to_uint32
    ),
    (
        usize,
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uintptr_t,
        sys::scm_to_uintptr_t
    ),
]);
#[cfg(target_pointer_width = "64")]
impl_scm_ty_for_int!([
    (
        u64,
        usize,
        sys::scm_is_unsigned_integer,
        sys::scm_from_uint64,
        sys::scm_to_uint64,
    ),
    (
        i64,
        isize,
        sys::scm_is_signed_integer,
        sys::scm_from_int64,
        sys::scm_to_int64,
    ),
]);

/// Marker trait for types that can be used with the `#[optional]` attribute in [guile_fn]
pub trait OptionalScm
where
    Self: From<Option<Self::Inner>> + Into<Option<Self::Inner>>,
{
    type Inner: ScmTy;
}
impl<T> OptionalScm for Option<T>
where
    T: ScmTy,
{
    type Inner = T;
}

/// Marker trait for types that can be used with the `#[rest]` attribute in [guile_fn]
pub trait RestScm<'a>: From<Scm<'a>> {}
impl<'a> RestScm<'a> for Scm<'a> {}

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

    /// Assert that [Self::REQUIRED] and [Self::OPTIONAL] are convertible to [c_int]s.
    const _LENGTH_CHECK: () = {
        assert!(Self::REQUIRED <= c_int::MAX as usize);
        assert!(Self::OPTIONAL <= c_int::MAX as usize);
    };
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{fmt::Debug, thread},
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
    fn dead_scm_send() {
        with_guile(|api| {
            let t = api.make(true);
            let t = api.kill(t);
            thread::spawn(move || {
                with_guile(move |api| {
                    let t = api.revive(t);
                    assert_eq!(t.get::<bool>(), Some(true));
                });
            });
        });
    }

    trait ApiExt {
        fn test_ty<'id, T>(&'id self, _: T, _: T::Output) -> Scm<'id>
        where
            T: ScmTy,
            T::Output: Debug + PartialEq;

        fn test_ty_equal<'id, T>(&'id self, val: T) -> Scm<'id>
        where
            T: Clone + Debug + PartialEq + ScmTy<Output = T>,
        {
            let scm = self.test_ty(val.clone(), val);
            assert!(scm.is_eqv(&scm));
            scm
        }
    }
    impl ApiExt for Api {
        fn test_ty<'id, T>(&'id self, val: T, output: T::Output) -> Scm<'id>
        where
            T: ScmTy,
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
            api.test_ty_equal(true);
            api.test_ty_equal(false);
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn char_conversion() {
        with_guile(|api| {
            api.test_ty_equal(char::MIN);
            api.test_ty_equal(char::MAX);
            ('a'..='z')
                .into_iter()
                .map(|ch| api.test_ty_equal(ch))
                .for_each(drop);
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_conversion() {
        (with_guile(|api| {
            let mut hello_world = AllocVec::new_in(CAllocator);
            hello_world.extend(b"hello world");
            api.test_ty(
                "hello world",
                Ok(unsafe { string::String::from_utf8_unchecked(hello_world) }),
            );

            let empty = AllocVec::new_in(CAllocator);
            api.test_ty(
                "",
                Ok(unsafe { string::String::from_utf8_unchecked(empty) }),
            );
        }));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_conversion() {
        // let x = true;

        macro_rules! test_ty {
            ($api:expr, [ $($ty:ty),+ $(,)? ]) => {
                $(test_ty!($api, $ty);)+
            };
            ($api:expr, $ty:ty) => {
                $api.test_ty_equal(<$ty>::MIN);
                let scm = $api.test_ty_equal(<$ty>::MAX);
                assert!(scm.is_number());
            };
        }
        with_guile(|api| {
            test_ty!(api, [c_double, i8, i16, i32, isize, u8, u16, u32, usize]);
            #[cfg(target_pointer_width = "64")]
            test_ty!(api, [i64, u64]);
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_ord() {
        with_guile(|api| {
            let [ref one, ref two, ref three] =
                (1..=3).map(|i| api.make(i)).collect::<Vec<_>>()[..]
            else {
                unreachable!()
            };

            assert!(one < two);
            assert!(one < three);
            assert!(one <= one);
            assert!(one <= two);
            assert!(one <= three);
            assert!(three > two);
            assert!(three > one);
            assert!(three >= two);
            assert!(three >= one);
            assert!(three >= three);
        });
    }
}
