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
        Api, Scm, ScmTy,
        sys::{scm_car, scm_cdr, scm_list_p, scm_null_p},
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
    },
};

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct List<'id, T>
where
    T: ScmTy<'id>,
{
    pair: Scm<'id>,
    _marker: PhantomData<T>,
}
impl<'id, T> ScmTy<'id> for List<'id, T>
where
    T: ScmTy<'id>,
{
    type Output = Self;

    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "(list {})",
            BStr::new(T::type_name().as_ref().to_bytes())
        ))
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(c"list"))
    }
    fn construct(self) -> Scm<'id> {
        self.pair
    }
    fn predicate(_: &Api, scm: &Scm) -> bool {
        unsafe { Scm::from_ptr(scm_list_p(scm.as_ptr())) }.is_true()
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        Self {
            pair: unsafe { scm.cast_lifetime() },
            _marker: PhantomData,
        }
    }
}
impl<'id, T> Iterator for List<'id, T>
where
    T: ScmTy<'id>,
{
    type Item = Result<T::Output, Scm<'id>>;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { Scm::from_ptr(scm_null_p(self.pair.as_ptr())) }.is_true() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr]
                .map(|accessor| unsafe { Scm::from_ptr(accessor(self.pair.as_ptr())) });
            self.pair = cdr;

            Some(car.get::<T>().ok_or(car))
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_iter() {
        with_guile(|api| {
            assert_eq!(
                api.eval_c(c"'(1 2 3)")
                    .get::<List<u32>>()
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
                [1, 2, 3]
            );
        })
        .unwrap();
    }
}
