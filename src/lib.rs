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
        ops::Not,
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
        Self::from(unsafe { sys::scm_eq_p(self.as_ptr(), r.as_ptr()) }).is_true()
    }

    /// Check equality with `eqv?` semantics
    pub fn is_eqv(&self, r: &Self) -> bool {
        Self::from(unsafe { sys::scm_eqv_p(self.as_ptr(), r.as_ptr()) }).is_true()
    }

    /// Check equality with `equal?` semantics
    pub fn is_equal(&self, r: &Self) -> bool {
        Self::from(unsafe { sys::scm_equal_p(self.as_ptr(), r.as_ptr()) }).is_true()
    }
}
impl From<crate::sys::SCM> for Scm<'_> {
    fn from(scm: crate::sys::SCM) -> Self {
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
            Some(Self::from(unsafe { sys::scm_not(self.as_ptr()) }))
        } else {
            None
        }
    }
}

pub trait ScmTy: Sized {
    type Output;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id>;
    fn predicate(_: &Api, _: &Scm) -> bool;
    /// Exract [Self::Output] from a scm.
    ///
    /// # Safety
    ///
    /// This function must be safe if [Self::predicate] returns [true].
    unsafe fn get_unchecked(_: &Api, _: &Scm) -> Self::Output;
}
impl ScmTy for bool {
    type Output = Self;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        let scm = match self {
            true => unsafe { crate::sys::SCM_BOOL_T },
            false => unsafe { crate::sys::SCM_BOOL_F },
        };

        Scm::from(scm)
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::scm_is_bool(scm.as_ptr()) }
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self {
        unsafe { crate::sys::scm_to_bool(scm.as_ptr()) }
    }
}
impl ScmTy for char {
    type Output = Self;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        Scm::from(unsafe { sys::scm_integer_to_char(sys::scm_from_uint32(u32::from(self))) })
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { sys::gargoyle_reexports_scm_is_true(sys::scm_char_p(scm.as_ptr())) }
    }

    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> char {
        char::from_u32(unsafe { sys::scm_to_uint32(sys::scm_char_to_integer(scm.as_ptr())) })
            .expect("Guile characters should return valid rust characters.")
    }
}
impl ScmTy for &str {
    type Output = Result<string::String<AllocVec<u8, CAllocator>>, AllocError>;

    fn construct<'id>(self, _: &'id Api) -> Scm<'id> {
        let scm = unsafe { crate::sys::scm_from_utf8_stringn(self.as_ptr().cast(), self.len()) };
        Scm::from(scm)
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
                Scm::from(unsafe { ($to_scm)(self) })
            }
            fn predicate(_: &Api, scm: &Scm) -> bool {
                unsafe {
                    ($predicate)(
                        scm.as_ptr(),
                        <$ptr>::MIN,
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
        fn test_ty<T>(&self, _: T, _: T::Output) -> Scm
        where
            T: ScmTy,
            T::Output: Debug + PartialEq;

        fn test_ty_equal<T>(&self, val: T)
        where
            T: Clone + Debug + PartialEq + ScmTy<Output = T>,
        {
            let scm = self.test_ty(val.clone(), val);
            assert!(scm.is_eqv(&scm));
        }
    }
    impl ApiExt for Api {
        fn test_ty<T>(&self, val: T, output: T::Output) -> Scm
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
            ('a'..='z').into_iter().for_each(|ch| api.test_ty_equal(ch));
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn string_conversion() {
        with_guile(|api| {
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
        });
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn int_conversion() {
        macro_rules! test_ty {
            ($api:expr, [ $($ty:ty),+ $(,)? ]) => {
                $(test_ty!($api, $ty);)+
            };
            ($api:expr, $ty:ty) => {
                $api.test_ty_equal(<$ty>::MIN);
                $api.test_ty_equal(<$ty>::MAX);
            };
        }
        with_guile(|api| {
            test_ty!(api, [i8, i16, i32, isize, u8, u16, u32, usize]);
            #[cfg(target_pointer_width = "64")]
            test_ty!(api, [i64, u64]);
        });
    }
}
