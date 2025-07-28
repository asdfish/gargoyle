// use {crate::Api, std::marker::PhantomData};

// impl Api {
//     fn scope<'id>(&'id self) -> Scope<'id> {
//         unsafe { Scope::new_unchecked() }
//     }
// }

// struct Scope<'id> {
//     _marker: PhantomData<&'id ()>,
// }
// impl Scope<'_> {
//     /// # Safety
//     ///
//     /// This function may only be called in guile mode.
//     unsafe fn new_unchecked() -> Self {
//         unsafe {
//             crate::sys::scm_dynwind_begin(0);
//         }

//         Self {
//             _marker: PhantomData,
//         }
//     }
// }
