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
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        sys::{
            SCM, scm_car, scm_cdr, scm_cons, scm_copy_tree, scm_is_pair, scm_set_car_x,
            scm_set_cdr_x,
        },
        utils::{CowCStrExt, c_predicate},
    },
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
    },
};

#[repr(transparent)]
pub struct Pair<'gm, L, R> {
    scm: SCM,
    _marker: PhantomData<&'gm (L, R)>,
}
impl<'gm, L, R> Pair<'gm, L, R>
where
    L: for<'a> TryFromScm<'a>,
    R: for<'a> TryFromScm<'a>,
{
    pub fn into_tuple(self) -> (L, R) {
        let guile = unsafe { Guile::new_unchecked() };
        (
            unsafe { L::from_scm_unchecked(Scm::from_ptr(scm_cdr(self.scm), &guile), &guile) },
            unsafe { R::from_scm_unchecked(Scm::from_ptr(scm_cdr(self.scm), &guile), &guile) },
        )
    }
}
impl<'gm, L, R> Pair<'gm, L, R> {
    pub fn as_car<'a>(&'a self) -> Ref<'a, 'gm, L> {
        unsafe { Ref::new_unchecked(scm_car(self.scm)) }
    }
    pub fn as_cdr<'a>(&'a self) -> Ref<'a, 'gm, R> {
        unsafe { Ref::new_unchecked(scm_cdr(self.scm)) }
    }
    pub fn as_mut_car<'a>(&'a mut self) -> RefMut<'a, 'gm, L> {
        unsafe { RefMut::new_unchecked(scm_car(self.scm)) }
    }
    pub fn as_mut_cdr<'a>(&'a mut self) -> RefMut<'a, 'gm, R> {
        unsafe { RefMut::new_unchecked(scm_cdr(self.scm)) }
    }
}
impl<L, R> Clone for Pair<'_, L, R> {
    fn clone(&self) -> Self {
        Pair {
            scm: unsafe { scm_copy_tree(self.scm) },
            _marker: PhantomData,
        }
    }
}
impl<'gm, L, R> Pair<'gm, L, R>
where
    L: ToScm<'gm>,
    R: ToScm<'gm>,
{
    pub fn new(car: L, cdr: R, guile: &'gm Guile) -> Self {
        let car = car.to_scm(guile);
        let cdr = cdr.to_scm(guile);
        Pair {
            scm: unsafe { scm_cons(car.as_ptr(), cdr.as_ptr()) },
            _marker: PhantomData,
        }
    }
}
impl<L, R> Pair<'_, L, R>
where
    L: for<'gm> ToScm<'gm>,
    R: for<'gm> ToScm<'gm>,
{
    pub fn set_car(&mut self, l: L) {
        let guile = unsafe { Guile::new_unchecked() };
        unsafe {
            scm_set_car_x(self.scm, l.to_scm(&guile).as_ptr());
        }
    }
    pub fn set_cdr(&mut self, r: R) {
        let guile = unsafe { Guile::new_unchecked() };
        unsafe {
            scm_set_cdr_x(self.scm, r.to_scm(&guile).as_ptr());
        }
    }
}
unsafe impl<L, R> ReprScm for Pair<'_, L, R> {}
impl<'gm, L, R> ToScm<'gm> for Pair<'gm, L, R> {
    fn to_scm(self, guile: &'gm Guile) -> Scm<'gm> {
        Scm::from_ptr(self.scm, guile)
    }
}
impl<'gm, L, R> TryFromScm<'gm> for Pair<'gm, L, R>
where
    L: for<'a> TryFromScm<'a>,
    R: for<'a> TryFromScm<'a>,
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
        c_predicate(|| unsafe { scm_is_pair(pair) })
            // SAFETY: the previous condition should short circuit if it is not a pair, making these safe
            && L::predicate(&Scm::from_ptr(unsafe { scm_car(pair) }, guile), guile)
            && R::predicate(&Scm::from_ptr(unsafe { scm_cdr(pair) }, guile), guile)
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm: scm.as_ptr(),
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
            assert_eq!(pair.as_car().into_inner(), 1);
            assert_eq!(pair.as_cdr().into_inner(), 2);

            pair.set_car(2);
            assert_eq!(pair.as_car().into_inner(), 2);

            let mut pair = Pair::new(1, Pair::new(2, 3, guile), guile);
            pair.as_mut_cdr().set_car(3);
            assert_eq!(pair.as_cdr().as_car().into_inner(), 3);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn pair_clone() {
        with_guile(|guile| {
            let mut pair = Pair::new(1, Pair::new(2, 3, guile), guile);
            let copy = pair.clone();
            pair.as_mut_cdr().set_car(3);
            assert_eq!(pair.as_cdr().as_car().into_inner(), 3);
            assert_eq!(copy.as_cdr().as_car().into_inner(), 2);
        })
        .unwrap();
    }
}
