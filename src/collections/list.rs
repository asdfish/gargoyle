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
    std::marker::PhantomData,
};

#[derive(Debug)]
#[repr(transparent)]
pub struct List<'gm, T>
where
    T: ToScm<'gm>,
{
    scm: Scm<'gm>,
    _marker: PhantomData<T>,
}
unsafe impl<'gm, T> ReprScm for List<'gm, T> where T: ToScm<'gm> {}
impl<'gm, T> List<'gm, T>
where
    T: ToScm<'gm>,
{
    pub fn new<I>(iter: I, guile: &'gm Guile) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            scm: Scm::from_ptr(
                iter.into_iter()
                    .fold(unsafe { SCM_EOL }, |cdr, car| unsafe {
                        scm_cons(car.to_scm(guile).as_ptr(), cdr)
                    }),
                guile,
            ),
            _marker: PhantomData,
        }
    }
}
impl<T> PartialEq for List<'_, T>
where
    T: for<'a> ToScm<'a>,
{
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
        with_guile(|guile| assert_eq!(List::new([1, 2, 3], guile), List::new([1, 2, 3], guile)))
            .unwrap();
    }
}
