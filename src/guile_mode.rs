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
        sys::{scm_with_guile, scm_without_guile},
    },
    parking_lot::Mutex,
    std::{
        ffi::c_void,
        marker::PhantomData,
        sync::atomic::{self, AtomicBool},
    },
};

static INIT_LOCK: Mutex<()> = Mutex::new(());
thread_local! {
    static INIT: AtomicBool = const { AtomicBool::new(false) };
    static GUILE_MODE: AtomicBool = const { AtomicBool::new(false) };
}

struct CallbackData<T>
where
    T: GuileModeToggle + ?Sized,
{
    morphism: Option<T::Fn>,
    output: Option<T::Output>,
    _marker: PhantomData<T>,
}

/// # Safety
///
/// [Self::SCOPE] should change guile mode to [Self::GUILE_MODE_STATUS], where entering guile mode is true and so on.
unsafe trait GuileModeToggle {
    type Fn;
    type Output;

    const LOCK_INIT: bool;
    const GUILE_MODE_STATUS: bool;
    const SCOPE: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void;

    /// # Safety
    ///
    /// This should be safe to run if [GUILE_MODE] is [Self::GUILE_MODE_STATUS].
    unsafe fn eval(_: Self::Fn) -> Self::Output;
    unsafe extern "C" fn callback(ptr: *mut c_void) -> *mut c_void {
        INIT.with(|mode| mode.store(true, atomic::Ordering::Release));
        GUILE_MODE.with(|mode| mode.store(Self::GUILE_MODE_STATUS, atomic::Ordering::Release));

        let ptr = ptr.cast::<CallbackData<Self>>();
        if let Some(data) = unsafe { ptr.as_mut() } {
            if data.output.is_none() {
                data.output = data.morphism.take().map(|f| unsafe { Self::eval(f) });
            }
        }

        std::ptr::null_mut()
    }
    fn toggle(morphism: Self::Fn) -> Option<Self::Output> {
        if GUILE_MODE.with(|mode| mode.load(atomic::Ordering::Acquire)) == Self::GUILE_MODE_STATUS {
            Some(unsafe { Self::eval(morphism) })
        } else {
            let _lock = (!INIT.with(|init| init.load(atomic::Ordering::Acquire))
                && Self::LOCK_INIT)
                .then(|| INIT_LOCK.lock());

            let mut data = CallbackData::<Self> {
                morphism: Some(morphism),
                output: None,
                _marker: PhantomData,
            };

            unsafe { Self::SCOPE(Some(Self::callback), (&raw mut data).cast()) };

            GUILE_MODE.with(|mode| mode.store(!Self::GUILE_MODE_STATUS, atomic::Ordering::Release));
            data.output
        }
    }
}
struct WithGuile<F, O>
where
    F: for<'a> FnOnce(&'a mut Guile) -> O,
{
    _marker: PhantomData<(F, O)>,
}
unsafe impl<F, O> GuileModeToggle for WithGuile<F, O>
where
    F: for<'a> FnOnce(&'a mut Guile) -> O,
{
    type Fn = F;
    type Output = O;

    const LOCK_INIT: bool = true;
    const GUILE_MODE_STATUS: bool = true;
    const SCOPE: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void = scm_with_guile;

    unsafe fn eval(f: Self::Fn) -> Self::Output {
        f(&mut unsafe { Guile::new_unchecked() })
    }
}

pub fn with_guile<F, O>(f: F) -> Option<O>
where
    F: for<'a> FnOnce(&'a mut Guile) -> O,
{
    WithGuile::toggle(f)
}

struct WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    _marker: PhantomData<(F, O)>,
}
unsafe impl<F, O> GuileModeToggle for WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    type Fn = F;
    type Output = O;

    const LOCK_INIT: bool = false;
    const GUILE_MODE_STATUS: bool = false;
    const SCOPE: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void = scm_without_guile;

    unsafe fn eval(f: Self::Fn) -> Self::Output {
        f()
    }
}
impl Guile {
    pub fn block_on<F, O>(&mut self, f: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::toggle(f).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use {super::*, itertools::Itertools, std::thread};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn multithreading() {
        fn spawn_with_guile() -> thread::JoinHandle<()> {
            thread::spawn(|| with_guile(|_| with_guile(|_| {}).unwrap()).unwrap())
        }
        fn spawn_without_guile() -> thread::JoinHandle<()> {
            thread::spawn(|| {
                with_guile(|guile| guile.block_on(|| with_guile(|_| {})).unwrap()).unwrap()
            })
        }

        [spawn_with_guile, spawn_without_guile]
            .into_iter()
            .tuple_combinations::<(_, _)>()
            .for_each(|(l, r)| {
                [l, r]
                    .map(|spawn| spawn())
                    .into_iter()
                    .map(|thread| thread.join())
                    .collect::<Result<_, _>>()
                    .unwrap()
            });
    }
}
