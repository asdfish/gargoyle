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
        collections::{
            byte_vector::{ByteVector, ByteVectorType},
            char_set::CharSet,
            vector::Vector,
        },
        hook::Hook,
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        subr::Proc,
        sys::{
            SCM, SCM_EOL, scm_car, scm_cdr, scm_char_set_to_list, scm_cons, scm_hook_to_list,
            scm_list_p, scm_vector_to_list,
        },
        utils::{CowCStrExt, scm_predicate},
    },
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        iter::{self, FusedIterator},
        marker::PhantomData,
    },
};

/// Create a list in the order provided.
///
/// The first argument is a [Guile] reference and the rest would be the items of the list.
///
/// # Examples
///
/// ```
/// # use gargoyle::{list, with_guile, collections::list::List, string::String};
/// # #[cfg(not(miri))]
/// with_guile(|guile| {
///    assert_eq!(unsafe { String::from_str("'(1 2 3)", guile).eval::<List<i32>>() }, Ok(list!(guile, 1, 2, 3)));
/// }).unwrap();
/// ```
#[macro_export]
macro_rules! list {
    ($guile:expr, $($i:expr),+ $(,)?) => {
        {
            let guile: &$crate::Guile = $guile;
            unsafe {
                <$crate::collections::list::List<_> as $crate::reference::ReprScm>::from_ptr(
                    $crate::sys::scm_list_n(
                        $($crate::scm::ToScm::to_scm($i, guile).as_ptr(),)+
                            $crate::sys::SCM_UNDEFINED,
                    )
                )
            }
        }
    };
}

#[derive(Debug)]
#[repr(transparent)]
pub struct List<'gm, T> {
    pub(crate) scm: Scm<'gm>,
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
        T: ToScm<'gm>,
    {
        let mut list = Self::new(guile);
        list.extend(iter);
        list
    }
    pub fn push_front(&mut self, item: T)
    where
        T: ToScm<'gm>,
    {
        self.extend(iter::once(item));
    }

    pub fn is_empty(&self) -> bool {
        self.scm.is_eol()
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, 'gm, T> {
        Iter {
            car: self.scm.as_ptr(),
            _marker: PhantomData,
        }
    }
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, 'gm, T> {
        IterMut {
            car: self.scm.as_ptr(),
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> Extend<T> for List<'gm, T>
where
    T: ToScm<'gm>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let pair = iter.into_iter().fold(self.scm.as_ptr(), |cdr, car| unsafe {
            scm_cons(car.to_scm(guile).as_ptr(), cdr)
        });
        self.scm = unsafe { Scm::from_ptr_unchecked(pair) };
    }
}
impl<'gm, T> From<ByteVector<'gm, T>> for List<'gm, T>
where
    T: ByteVectorType,
{
    fn from(vector: ByteVector<'gm, T>) -> Self {
        List {
            scm: unsafe { Scm::from_ptr_unchecked(T::TO_LIST(vector.scm.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm> From<CharSet<'gm>> for List<'gm, char> {
    fn from(chrs: CharSet<'gm>) -> Self {
        List {
            scm: unsafe { Scm::from_ptr_unchecked(scm_char_set_to_list(chrs.0.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm, const ARITY: usize> From<Hook<'gm, ARITY>> for List<'gm, Proc<'gm>> {
    fn from(hook: Hook<'gm, ARITY>) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr_unchecked(scm_hook_to_list(hook.0.as_ptr())) },
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> From<Vector<'gm, T>> for List<'gm, T> {
    fn from(vector: Vector<'gm, T>) -> Self {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Self {
            scm: Scm::from_ptr(unsafe { scm_vector_to_list(vector.scm.as_ptr()) }, guile),
            _marker: PhantomData,
        }
    }
}
impl<'gm, T> IntoIterator for List<'gm, T>
where
    T: TryFromScm<'gm>,
{
    type Item = T;
    type IntoIter = IntoIter<'gm, T>;

    fn into_iter(self) -> IntoIter<'gm, T> {
        IntoIter(self)
    }
}
impl<'a, 'gm, T> IntoIterator for &'a List<'gm, T>
where
    T: 'gm,
{
    type Item = Ref<'a, 'gm, T>;
    type IntoIter = Iter<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, 'gm, T> IntoIterator for &'a mut List<'gm, T>
where
    T: 'gm,
{
    type Item = RefMut<'a, 'gm, T>;
    type IntoIter = IterMut<'a, 'gm, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
impl<T> PartialEq for List<'_, T> {
    fn eq(&self, r: &Self) -> bool {
        self.scm.is_equal(&r.scm)
    }
}
impl<'gm, T> ToScm<'gm> for List<'gm, T> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm, T> TryFromScm<'gm> for List<'gm, T>
where
    T: TryFromScm<'gm>,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!("(list {})", T::type_name().display()))
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(c"list"))
    }
    fn predicate(scm: &Scm<'gm>, guile: &'gm Guile) -> bool {
        scm_predicate(unsafe { scm_list_p(scm.as_ptr()) }) && {
            IntoIter(List {
                scm: unsafe { scm.copy_unchecked() },
                _marker: PhantomData::<Scm>,
            })
            .all(|i| T::predicate(&i, guile))
        }
    }
    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

pub struct IntoIter<'gm, T>(List<'gm, T>);
impl<'gm, T> From<IntoIter<'gm, T>> for List<'gm, T> {
    fn from(IntoIter(lst): IntoIter<'gm, T>) -> List<'gm, T> {
        lst
    }
}
impl<'gm, T> FusedIterator for IntoIter<'gm, T> where T: TryFromScm<'gm> {}
impl<'gm, T> Iterator for IntoIter<'gm, T>
where
    T: TryFromScm<'gm>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.0.scm.is_eol() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr]
                .map(|morphism| unsafe { morphism(self.0.scm.as_ptr()) })
                .map(|ptr| unsafe { Scm::from_ptr_unchecked(ptr) });
            self.0.scm = cdr;

            let guile = unsafe { Guile::new_unchecked_ref() };
            Some(unsafe { T::from_scm_unchecked(car, guile) })
        }
    }
}

#[derive(Clone, Copy)]
pub struct Iter<'a, 'gm, T> {
    car: SCM,
    _marker: PhantomData<&'a &'gm T>,
}
impl<T> FusedIterator for Iter<'_, '_, T> {}
impl<'a, 'gm, T> Iterator for Iter<'a, 'gm, T> {
    type Item = Ref<'a, 'gm, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { Scm::from_ptr_unchecked(self.car) }.is_eol() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr].map(|morphism| unsafe { morphism(self.car) });
            self.car = cdr;

            Some(unsafe { Ref::new_unchecked(car) })
        }
    }
}

#[derive(Clone, Copy)]
pub struct IterMut<'a, 'gm, T> {
    car: SCM,
    _marker: PhantomData<&'a &'gm T>,
}
impl<T> FusedIterator for IterMut<'_, '_, T> {}
impl<'a, 'gm, T> Iterator for IterMut<'a, 'gm, T> {
    type Item = RefMut<'a, 'gm, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { Scm::from_ptr_unchecked(self.car) }.is_eol() {
            None
        } else {
            let [car, cdr] = [scm_car, scm_cdr].map(|morphism| unsafe { morphism(self.car) });
            self.car = cdr;

            Some(unsafe { RefMut::new_unchecked(car) })
        }
    }
}

#[repr(transparent)]
pub struct Null<'gm>(Scm<'gm>);
impl<'gm> Null<'gm> {
    pub fn new(guile: &'gm Guile) -> Self {
        Self(Scm::from_ptr(unsafe { SCM_EOL }, guile))
    }
}
unsafe impl ReprScm for Null<'_> {}
impl<'gm> TryFromScm<'gm> for Null<'gm> {
    fn type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"null")
    }
    fn predicate(scm: &Scm<'gm>, _: &'gm Guile) -> bool {
        scm.is_eol()
    }
    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self(scm)
    }
}
impl<'gm> ToScm<'gm> for Null<'gm> {
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{alloc::CAllocator, string::String, with_guile},
        allocator_api2::vec,
        std::collections::HashSet,
    };

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_construction() {
        with_guile(|guile| {
            assert_eq!(
                List::from_iter([1, 2, 3], guile),
                List::from_iter([1, 2, 3], guile)
            );

            assert_eq!(List::from(Hook::<0>::new(guile)).iter().count(), 0);
            assert_eq!(
                List::from(ByteVector::from(vec![in CAllocator; 1; 4]))
                    .into_iter()
                    .collect::<Vec<_>>(),
                [1; 4]
            );
            const TEXT: &str = "thequickbrownfoxjumpsoverthelazydog";
            assert_eq!(
                List::from(CharSet::from(String::from_str(TEXT, guile)))
                    .into_iter()
                    .collect::<HashSet<_>>(),
                HashSet::from_iter(TEXT.chars())
            );
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_into_iter() {
        with_guile(|guile| {
            let mut list = List::from_iter('a'..='c', guile);
            assert_eq!(
                list.iter().map(Ref::into_inner).collect::<Vec<_>>(),
                ['c', 'b', 'a'],
            );
            assert_eq!(
                list.iter_mut().map(RefMut::into_inner).collect::<Vec<_>>(),
                ['c', 'b', 'a'],
            );
            assert_eq!(list.into_iter().collect::<Vec<_>>(), ['c', 'b', 'a'],);
        })
        .unwrap();
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn list_is_empty() {
        with_guile(|guile| {
            let mut lst = List::new(guile);
            assert!(lst.is_empty());
            lst.extend([1]);
            assert!(!lst.is_empty());

            assert!(List::<i32>::new(guile).is_empty());
            assert!(List::<i32>::from_iter([], guile).is_empty());
        })
        .unwrap();
    }
}
