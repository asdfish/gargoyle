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
        num::{Num, UInt},
        scm::Scm,
        sys::{scm_copy_random_state, scm_random, scm_seed_to_random_state},
    },
    std::ops::RangeTo,
};

pub struct Generator<'gm, T>
where
    T: UInt<'gm>,
{
    random_state: Scm<'gm>,
    end: RangeTo<T>,
}

// pub struct Generator<'guile_mode, T>(Scm<'guile_mode>);
impl<'gm, T> Clone for Generator<'gm, T>
where
    T: UInt<'gm>,
{
    fn clone(&self) -> Self {
        Self {
            random_state: unsafe {
                Scm::from_ptr_unchecked(scm_copy_random_state(self.random_state.as_ptr()))
            },
            end: self.end.clone(),
        }
    }
}
impl<'gm, T> Generator<'gm, T>
where
    T: UInt<'gm>,
{
    pub fn new<S>(seed: S, end: RangeTo<T>, guile: &'gm Guile) -> Self
    where
        S: Num<'gm>,
    {
        Self {
            random_state: Scm::from_ptr(
                unsafe { scm_seed_to_random_state(seed.to_scm(guile).as_ptr()) },
                guile,
            ),
            end,
        }
    }
}
impl<T> Iterator for Generator<'_, T>
where
    T: for<'gm> UInt<'gm>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let guile = unsafe { Guile::new_unchecked() };
        T::try_from_scm(
            Scm::from_ptr(
                unsafe {
                    scm_random(
                        self.end.end.to_scm(&guile).as_ptr(),
                        self.random_state.as_ptr(),
                    )
                },
                &guile,
            ),
            &guile,
        )
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn clone() {
        with_guile(|guile| {
            const RANGE: RangeTo<u32> = ..100;
            let mut a = Generator::new(0, RANGE, &guile);
            let mut b = Generator::new(0, RANGE, &guile);
            let mut c = a.clone();

            (0..=10_000).for_each(|_| {
                let [a, b, c] = [&mut a, &mut b, &mut c]
                    .map(|g| g.next())
                    .map(Option::unwrap);

                assert_eq!(a, b);
                assert_eq!(b, c);
            })
        });
    }
}
