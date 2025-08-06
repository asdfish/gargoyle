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
        scm::{Scm, ToScm, TryFromScm},
        sys::{SCM_HOOK_ARITY, SCM_HOOKP, scm_hook_empty_p, scm_make_hook},
        utils::{c_predicate, scm_predicate},
    },
    std::{borrow::Cow, ffi::CStr, marker::PhantomData},
};

trait Tuple<'gm>: ToScm<'gm> + TryFromScm<'gm> {
    const ARITY: usize;
}
macro_rules! impl_tuple_for {
    () => {
        impl<'gm> $crate::hook::Tuple<'gm> for ()
        {
            const ARITY: usize = 0;
        }
    };
    ($car:ident $(, $($cdr:ident),+ $(,)?)?) => {
        impl<'gm, $car $(, $($cdr),+)?> $crate::hook::Tuple<'gm> for ($car, $($($cdr),+)?)
        where
            $car: ToScm<'gm> + TryFromScm<'gm>,
        $($($cdr: ToScm<'gm> + TryFromScm<'gm>),+)?
        {
            const ARITY: usize = { 1 $($(+ { const $cdr: usize = 1; $cdr })+)? };
        }

        impl_tuple_for!($($($cdr),+)?);
    }
}
impl_tuple_for!(A, B, C, D, E, F, G, H, I, J, K, L);

#[repr(transparent)]
pub struct Hook<'gm, Args>
where
    Args: Tuple<'gm>,
{
    scm: Scm<'gm>,
    _marker: PhantomData<Args>,
}
impl<'gm, Args> Hook<'gm, Args>
where
    Args: Tuple<'gm>,
{
    pub fn new(guile: &'gm Guile) -> Self {
        Self {
            scm: Scm::from_ptr(
                unsafe { scm_make_hook(Args::ARITY.to_scm(guile).as_ptr()) },
                guile,
            ),
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        scm_predicate(unsafe { scm_hook_empty_p(self.scm.as_ptr()) })
    }
}
unsafe impl<'gm, Args> ReprScm for Hook<'gm, Args> where Args: Tuple<'gm> {}
impl<'gm, Args> ToScm<'gm> for Hook<'gm, Args>
where
    Args: Tuple<'gm>,
{
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}

impl<'gm, Args> TryFromScm<'gm> for Hook<'gm, Args>
where
    Args: Tuple<'gm>,
{
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"hook")
    }

    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        c_predicate(unsafe { SCM_HOOKP(scm.as_ptr()) })
            && usize::try_from(unsafe { SCM_HOOK_ARITY(scm.as_ptr()) })
                .map(|arity| arity == Args::ARITY)
                .unwrap_or_default()
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
    fn hook_is_empty() {
        with_guile(|guile| {
            let hook = Hook::<()>::new(guile);
            assert!(hook.is_empty());
        })
        .unwrap();
    }
}
