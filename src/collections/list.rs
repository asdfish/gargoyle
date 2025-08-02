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
        reference::ReprScm,
        scm::{Scm, ToScm},
        sys::{SCM_EOL, scm_cons},
    },
    std::{iter, marker::PhantomData},
};

#[derive(Debug)]
#[repr(transparent)]
pub struct List<'gm, T> {
    scm: Scm<'gm>,
    _marker: PhantomData<T>,
}
unsafe impl<'gm, T> ReprScm for List<'gm, T> {}
impl<'gm, T> List<'gm, T> {
    pub fn new(guile: &'gm Guile) -> Self {
        Self {
            scm: Scm::from_ptr(unsafe { SCM_EOL }, guile),
            _marker: PhantomData,
        }
    }

    /// Create a list in reverse order of the iterator.
    pub fn from_iter<I>(iter: I, guile: &'gm Guile) -> Self
    where
        I: IntoIterator<Item = T>,
        T: for<'a> ToScm<'a>,
    {
        let mut list = Self::new(guile);
        list.extend(iter);
        list
    }
    pub fn push_front(&mut self, item: T)
    where
        T: for<'a> ToScm<'a>,
    {
        self.extend(iter::once(item));
    }
}
impl<T> Extend<T> for List<'_, T>
where
    T: for<'a> ToScm<'a>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let guile = unsafe { Guile::new_unchecked() };
        let car = iter.into_iter().fold(self.scm.as_ptr(), |cdr, car| unsafe {
            scm_cons(car.to_scm(&guile).as_ptr(), cdr)
        });
        self.scm = unsafe { Scm::from_ptr_unchecked(car) };
    }
}
impl<T> PartialEq for List<'_, T> {
    fn eq(&self, r: &Self) -> bool {
        self.scm.is_equal(&r.scm)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_construction() {
        with_guile(|guile| {
            assert_eq!(
                List::from_iter([1, 2, 3], guile),
                List::from_iter([1, 2, 3], guile)
            )
        })
        .unwrap();
    }
}
