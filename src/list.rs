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
        sys::{SCM_EOL, scm_car, scm_cdr, scm_cons, scm_list_p, scm_null_p},
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
    },
};

impl Api {
    /// Create a list.
    ///
    /// The contents of the list will be in reverse order of the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::with_guile;
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///      let list = api.make_list([1, 2, 3])
    ///           .into_iter()
    ///           .collect::<Vec<_>>();
    ///      assert_eq!(list, [3, 2, 1]);
    /// }).unwrap();
    /// ```
    pub fn make_list<'id, I, T>(&'id self, iter: I) -> List<'id, T>
    where
        I: IntoIterator<Item = T>,
        T: ScmTy<'id>,
    {
        let list = iter
            .into_iter()
            .map(T::construct)
            .map(|scm| unsafe { scm.as_ptr() })
            .fold(unsafe { SCM_EOL }, |cdr, car| unsafe { scm_cons(car, cdr) });
        let list = unsafe { Scm::from_ptr(list) };
        List {
            pair: list,
            _marker: PhantomData,
        }
    }
}

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
        unsafe { Scm::from_ptr(scm_list_p(scm.as_ptr())) }.is_true() && {
            // eagerly check all items for better error messages
            IntoIter::<'id, Scm>(List {
                pair: unsafe { scm.cast_lifetime() },
                _marker: PhantomData,
            })
            .all(|i| i.is::<T>())
        }
    }
    unsafe fn get_unchecked(_: &Api, scm: &Scm) -> Self::Output {
        Self {
            pair: unsafe { scm.cast_lifetime() },
            _marker: PhantomData,
        }
    }
}
impl<'id, T> IntoIterator for List<'id, T>
where
    T: ScmTy<'id>,
{
    type Item = T::Output;
    type IntoIter = IntoIter<'id, T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

#[derive(Clone, Debug)]
pub struct IntoIter<'id, T>(List<'id, T>)
where
    T: ScmTy<'id>;
impl<'id, T> Iterator for IntoIter<'id, T>
where
    T: ScmTy<'id>,
{
    type Item = T::Output;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { Scm::from_ptr(scm_null_p(self.0.pair.as_ptr())) }.is_true() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr]
                .map(|morphism| unsafe { Scm::from_ptr(morphism(self.0.pair.as_ptr())) });
            self.0.pair = cdr;

            Some(unsafe { T::get_unchecked(&Api::new_unchecked(), &car) })
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[test]
    fn list_type() {
        assert_eq!(List::<'_, i32>::type_name().as_ref(), c"(list i32)");
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_iter() {
        with_guile(|api| {
            assert_eq!(
                api.eval_c(c"'(1 2 3)")
                    .get::<List<u32>>()
                    .unwrap()
                    .into_iter()
                    .collect::<Vec<_>>(),
                [1, 2, 3]
            );
        })
        .unwrap();
    }
}
