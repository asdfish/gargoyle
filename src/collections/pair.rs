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

//! Tuples in guile

use {
    crate::{
        Guile,
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        sys::{scm_car, scm_cdr, scm_cons, scm_is_pair, scm_set_car_x, scm_set_cdr_x},
        utils::{CowCStrExt, c_predicate},
    },
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
    },
};

/// Tuples with 2 elements.
#[repr(transparent)]
pub struct Pair<'gm, L, R> {
    scm: Scm<'gm>,
    _marker: PhantomData<(L, R)>,
}
impl<'gm, L, R> Pair<'gm, L, R>
where
    L: TryFromScm<'gm>,
    R: TryFromScm<'gm>,
{
    /// Convert a pair into a tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(Pair::new(1, 2, guile).to_tuple(), (1, 2));
    /// }).unwrap();
    /// ```
    pub fn to_tuple(self) -> (L, R) {
        let guile = unsafe { Guile::new_unchecked_ref() };
        (
            unsafe {
                L::from_scm_unchecked(Scm::from_ptr(scm_car(self.scm.as_ptr()), guile), guile)
            },
            unsafe {
                R::from_scm_unchecked(Scm::from_ptr(scm_cdr(self.scm.as_ptr()), guile), guile)
            },
        )
    }
}
impl<'gm, L, R> Pair<'gm, L, R> {
    /// Get a reference to the left of the pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(Pair::new(1, 2, guile).as_car().copied(), 1);
    /// }).unwrap();
    /// ```
    pub fn as_car<'a>(&'a self) -> Ref<'a, 'gm, L> {
        unsafe { Ref::new_unchecked(scm_car(self.scm.as_ptr())) }
    }
    /// Get a reference to the right of the pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(Pair::new(1, 2, guile).as_cdr().copied(), 2);
    /// }).unwrap();
    /// ```
    pub fn as_cdr<'a>(&'a self) -> Ref<'a, 'gm, R> {
        unsafe { Ref::new_unchecked(scm_cdr(self.scm.as_ptr())) }
    }
    /// Get a mutable reference to the left side of the pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut pair = Pair::new(Pair::new(1, 1, guile), 2, guile);
    ///     assert_eq!(pair.as_car().as_car().copied(), 1);
    ///     pair.as_mut_car().set_car(0);
    ///     assert_eq!(pair.as_car().as_car().copied(), 0);
    /// }).unwrap();
    /// ```
    pub fn as_mut_car<'a>(&'a mut self) -> RefMut<'a, 'gm, L> {
        unsafe { RefMut::new_unchecked(scm_car(self.scm.as_ptr())) }
    }
    /// Get a mutable reference to the right side of the pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{list, collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut pair = Pair::new(0, Pair::new(2, 2, guile), guile);
    ///     assert_eq!(pair.as_cdr().as_car().copied(), 2);
    ///     pair.as_mut_cdr().set_car(1);
    ///     assert_eq!(pair.as_cdr().as_car().copied(), 1);
    /// }).unwrap();
    /// ```
    pub fn as_mut_cdr<'a>(&'a mut self) -> RefMut<'a, 'gm, R> {
        unsafe { RefMut::new_unchecked(scm_cdr(self.scm.as_ptr())) }
    }
}
impl<'gm, L, R> Pair<'gm, L, R>
where
    L: ToScm<'gm>,
    R: ToScm<'gm>,
{
    /// Create a pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     assert_eq!(Pair::new(true, false, guile).to_tuple(), (true, false));
    /// }).unwrap();
    /// ```
    pub fn new(car: L, cdr: R, guile: &'gm Guile) -> Self {
        let car = car.to_scm(guile);
        let cdr = cdr.to_scm(guile);
        Pair {
            scm: Scm::from_ptr(unsafe { scm_cons(car.as_ptr(), cdr.as_ptr()) }, guile),
            _marker: PhantomData,
        }
    }
}
impl<'gm, L, R> Pair<'gm, L, R>
where
    L: ToScm<'gm>,
    R: ToScm<'gm>,
{
    /// Set the left side of a pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut pair = Pair::new(true, false, guile);
    ///     pair.set_car(false);
    ///     assert_eq!(pair.to_tuple(), (false, false));
    /// }).unwrap();
    /// ```
    pub fn set_car(&mut self, l: L) {
        let guile = unsafe { Guile::new_unchecked_ref() };
        unsafe {
            scm_set_car_x(self.scm.as_ptr(), l.to_scm(guile).as_ptr());
        }
    }
    /// Set the right side of a pair.
    ///
    /// # Examples
    ///
    /// ```
    /// # use garguile::{collections::pair::Pair, with_guile};
    /// # #[cfg(not(miri))]
    /// with_guile(|guile| {
    ///     let mut pair = Pair::new(true, false, guile);
    ///     pair.set_cdr(true);
    ///     assert_eq!(pair.to_tuple(), (true, true));
    /// }).unwrap();
    /// ```
    pub fn set_cdr(&mut self, r: R) {
        let guile = unsafe { Guile::new_unchecked_ref() };
        unsafe {
            scm_set_cdr_x(self.scm.as_ptr(), r.to_scm(guile).as_ptr());
        }
    }
}
unsafe impl<L, R> ReprScm for Pair<'_, L, R> {}
impl<'gm, L, R> ToScm<'gm> for Pair<'gm, L, R> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm, L, R> TryFromScm<'gm> for Pair<'gm, L, R>
where
    L: TryFromScm<'gm>,
    R: TryFromScm<'gm>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "({} . {})",
            L::type_name().display(),
            R::type_name().display()
        ))
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed(c"pair"))
    }
    fn predicate(scm: &Scm<'gm>, guile: &'gm Guile) -> bool {
        let pair = scm.as_ptr();
        // SAFETY: this should take everything
        c_predicate(unsafe { scm_is_pair(pair) })
            // SAFETY: the previous condition should short circuit if it is not a pair, making these safe
            && L::predicate(&Scm::from_ptr(unsafe { scm_car(pair) }, guile), guile)
            && R::predicate(&Scm::from_ptr(unsafe { scm_cdr(pair) }, guile), guile)
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn pair_construction() {
        with_guile(|guile| {
            let mut pair = Pair::new(1, 2, guile);
            assert_eq!(pair.as_car().copied(), 1);
            assert_eq!(pair.as_cdr().copied(), 2);

            pair.set_car(2);
            assert_eq!(pair.as_car().copied(), 2);

            let mut pair = Pair::new(1, Pair::new(2, 3, guile), guile);
            pair.as_mut_cdr().set_car(3);
            assert_eq!(pair.as_cdr().as_car().copied(), 3);
        })
        .unwrap();
    }
}
