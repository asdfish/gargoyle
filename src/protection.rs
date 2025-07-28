use {
    crate::{Api, with_guile},
    std::{ffi::c_void, marker::PhantomData, mem::ManuallyDrop, pin::Pin, ptr},
};

/// [with_guile] but with something that can protect things that implement [Drop].
pub fn with_guile_protected<F, O>(operation: F) -> Option<O>
where
    F: FnOnce(&mut Api, &Guardian) -> O,
{
    with_guile(|api| {
        let scope = unsafe { Guardian::new_unchecked() };
        operation(api, &scope)
    })
}

pub struct Guardian<'id> {
    _marker: PhantomData<&'id ()>,
}
impl<'id> Guardian<'id> {
    /// # Safety
    /// - This function may only be called in guile mode.
    /// - [Guardian] must be dropped.
    pub unsafe fn new_unchecked() -> Self {
        unsafe {
            crate::sys::scm_dynwind_begin(0);
        }

        Self {
            _marker: PhantomData,
        }
    }

    /// # Safety
    ///
    /// `ptr` must be of type [`ManuallyDrop<T>`]
    unsafe extern "C" fn protect_driver<T>(ptr: *mut c_void)
    where
        T: Drop,
    {
        let ptr = ptr.cast::<ManuallyDrop<T>>();

        // SAFETY: has valid alignment since it was made from a reference
        if let Some(ptr) = unsafe { ptr.as_mut() } {
            // SAFETY: we probably have read/write access and is not null
            unsafe {
                ManuallyDrop::drop(ptr);
            }
        }
    }

    /// Protect the pointer from unwinding.
    ///
    /// This does not add the protection to the scope of the object, it adds the protection to the scope that you call it in.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::with_guile_protected;
    /// # use std::{mem::ManuallyDrop, pin::Pin, sync::atomic::{self, AtomicU32}};
    /// # #[cfg(not(miri))]
    /// static COUNTER: AtomicU32 = AtomicU32::new(0) ;
    /// struct IncrCounter;
    /// impl Drop for IncrCounter {
    ///     fn drop(&mut self) { COUNTER.fetch_add(1, atomic::Ordering::Release); }
    /// }
    /// with_guile_protected(|_, g1| {
    ///     assert_eq!(0, COUNTER.load(atomic::Ordering::Acquire));
    ///     with_guile_protected(|_, _| {
    ///         let mut counter = ManuallyDrop::new(IncrCounter);
    ///         g1.protect(unsafe { Pin::new_unchecked(&mut counter) });
    ///     }); // counter gets dropped here
    ///     assert_eq!(1, COUNTER.load(atomic::Ordering::Acquire));
    /// });
    /// ```
    pub fn protect<'pin, T>(
        &'pin self,
        mut ptr: Pin<&'pin mut ManuallyDrop<T>>,
    ) -> Pin<&'pin mut ManuallyDrop<T>>
    where
        T: Drop,
    {
        let drop_ptr = ptr::from_mut(unsafe { ptr.as_mut().get_unchecked_mut() }).cast::<c_void>();
        // Guile should not know move the pointer and [protect_driver] does not move it.
        unsafe {
            crate::sys::scm_dynwind_rewind_handler(
                Some(Self::protect_driver::<T>),
                drop_ptr,
                crate::sys::SCM_F_WIND_EXPLICITLY,
            );
        }

        ptr
    }
}
impl Drop for Guardian<'_> {
    fn drop(&mut self) {
        // SAFETY: we are in guile mode and `&mut self` is proof of guile mode.
        unsafe {
            crate::sys::scm_dynwind_end();
        }
    }
}
