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
        sys::{SCM_EOL, scm_car, scm_cdr, scm_cons, scm_length, scm_list_p, scm_null_p},
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        iter::{ExactSizeIterator, FusedIterator},
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
        let mut lst = unsafe { List::new() };
        lst.extend(iter);
        lst
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct List<'id, T>
where
    T: ScmTy<'id>,
{
    pub(crate) pair: Scm<'id>,
    _marker: PhantomData<T>,
}
// `T` doesn't need to be clone since it gets constructed every time
// impl<'id, T> Clone for List<'id, T>
// where
//     T: ScmTy<'id>,
// {
//     fn clone(&self) -> Self {
//         Self {
//             pair: self.pair,
//             _marker: PhantomData,
//         }
//     }
// }
impl<'id, T> List<'id, T>
where
    T: ScmTy<'id>,
{
    /// # Safety
    ///
    /// The lifetime should be associated with the guile mode status.
    pub unsafe fn new() -> Self {
        Self {
            pair: unsafe { Scm::from_ptr(SCM_EOL) },
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        unsafe { Scm::from_ptr(scm_length(self.pair.as_ptr())) }
            .get::<usize>()
            .expect("list is too large")
    }
    pub fn is_empty(&self) -> bool {
        unsafe { Scm::from_ptr(scm_null_p(self.pair.as_ptr())) }.is_true()
    }

    pub fn front(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let api = unsafe { Api::new_unchecked() };
            Some(unsafe { T::get_unchecked(&api, Scm::from_ptr(scm_car(self.pair.as_ptr()))) })
        }
    }

    pub fn clear(&mut self) {
        self.pair = unsafe { Scm::from_ptr(SCM_EOL) };
    }
}
impl<'id, T> Extend<T> for List<'id, T>
where
    T: ScmTy<'id>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let lst = iter
            .into_iter()
            .fold(unsafe { self.pair.as_ptr() }, |cdr, car| unsafe {
                scm_cons(T::construct(car).as_ptr(), cdr)
            });
        self.pair = unsafe { Scm::from_ptr(lst) };
    }
}
impl<'id, T> ScmTy<'id> for List<'id, T>
where
    T: ScmTy<'id>,
{
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
                // SAFETY: we don't do any writing
                pair: unsafe { Scm::from_ptr(scm.as_ptr()).cast_lifetime() },
                _marker: PhantomData,
            })
            .all(|i| i.is::<T>())
        }
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self {
            pair: scm,
            _marker: PhantomData,
        }
    }
}
impl<'id, T> IntoIterator for List<'id, T>
where
    T: ScmTy<'id>,
{
    type Item = T;
    type IntoIter = IntoIter<'id, T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

#[derive(Debug)]
pub struct IntoIter<'id, T>(List<'id, T>)
where
    T: ScmTy<'id>;
impl<'id, T> IntoIter<'id, T>
where
    T: ScmTy<'id>,
{
    /// Get the list back out
    ///
    /// # Examples
    ///
    /// ```
    /// # use gargoyle::with_guile;
    /// # #[cfg(not(miri))]
    /// with_guile(|api| {
    ///     let mut iter = api.make_list::<_, i32>([1, 2, 3])
    ///         .into_iter();
    ///     assert_eq!(iter.next(), Some(3));
    ///     assert_eq!(iter.next(), Some(2));
    ///     let lst = iter.into_inner();
    ///     assert_eq!(lst.front(), Some(1));
    /// }).unwrap();
    /// ```
    pub fn into_inner(self) -> List<'id, T> {
        self.0
    }
}
impl<'id, T> ExactSizeIterator for IntoIter<'id, T> where T: ScmTy<'id> {}
impl<'id, T> FusedIterator for IntoIter<'id, T> where T: ScmTy<'id> {}
impl<'id, T> Iterator for IntoIter<'id, T>
where
    T: ScmTy<'id>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr]
                .map(|morphism| unsafe { Scm::from_ptr(morphism(self.0.pair.as_ptr())) });
            self.0.pair = cdr;

            Some(unsafe { T::get_unchecked(&Api::new_unchecked(), car) })
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.len();
        (len, Some(len))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::with_guile};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_len() {
        with_guile(|api| {
            assert_eq!(api.make_list([1, 2, 3]).len(), 3);
        })
        .unwrap();
    }

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
