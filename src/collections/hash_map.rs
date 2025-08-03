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
        collections::pair::Pair,
        reference::{Ref, RefMut, ReprScm},
        scm::{Scm, ToScm, TryFromScm},
        sys::{
            SCM, SCM_BOOL_F, SCM_UNDEFINED, scm_c_make_gsubr, scm_cdr, scm_hash_fold,
            scm_hash_table_p, scm_make_hash_table,
        },
        utils::CowCStrExt,
    },
    std::{
        borrow::Cow,
        ffi::{CStr, CString, c_void},
        marker::PhantomData,
    },
};

pub trait ScmPartialEq {
    /// Add `val` to `key` to `table` and return `val`. If it already exists, it would be overwritten.
    const SET: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM;
    /// Remove `key` from `table` and return its pair.
    const REMOVE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM;
    /// Get a handle from `key` in `table` or `#f` if it doesn't exist.
    const GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM;
    /// Get the handle or insert it.
    const CREATE_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM;
}

pub struct Eq;
impl ScmPartialEq for Eq {
    const SET: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        crate::sys::scm_hashq_set_x;
    const REMOVE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hashq_remove_x;
    const GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hashq_get_handle;
    const CREATE_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        crate::sys::scm_hashq_create_handle_x;
}

pub struct Eqv;
impl ScmPartialEq for Eqv {
    const SET: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        crate::sys::scm_hashv_set_x;
    const REMOVE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hashv_remove_x;
    const GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hashv_get_handle;
    const CREATE_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        crate::sys::scm_hashv_create_handle_x;
}

pub struct Equal;
impl ScmPartialEq for Equal {
    const SET: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        crate::sys::scm_hash_set_x;
    const REMOVE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hash_remove_x;
    const GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        crate::sys::scm_hash_get_handle;
    const CREATE_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        crate::sys::scm_hash_create_handle_x;
}

#[repr(transparent)]
pub struct HashMap<'gm, K, V, E = Equal>
where
    E: ScmPartialEq,
{
    scm: Scm<'gm>,
    _marker: PhantomData<(K, V, E)>,
}
impl<'gm, K, V, E> HashMap<'gm, K, V, E>
where
    E: ScmPartialEq,
{
    pub fn new(guile: &'gm Guile) -> Self {
        Self {
            scm: Scm::from_ptr(unsafe { scm_make_hash_table(SCM_UNDEFINED) }, guile),
            _marker: PhantomData,
        }
    }
    pub fn with_capacity(cap: usize, guile: &'gm Guile) -> Self {
        Self {
            scm: Scm::from_ptr(
                unsafe { scm_make_hash_table(cap.to_scm(guile).as_ptr()) },
                guile,
            ),
            _marker: PhantomData,
        }
    }

    pub fn get<'a>(&'a self, key: K) -> Option<Ref<'a, 'gm, V>>
    where
        K: TryFromScm<'gm> + ToScm<'gm> + 'gm,
        V: TryFromScm<'gm> + 'gm,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let handle = unsafe { E::GET_HANDLE(self.scm.as_ptr(), key.to_scm(guile).as_ptr()) };
        if Pair::<K, V>::predicate(&Scm::from_ptr(handle, guile), guile) {
            Some(unsafe { Ref::new_unchecked(scm_cdr(handle)) })
        } else {
            None
        }
    }
    pub fn get_mut<'a>(&'a mut self, key: K) -> Option<RefMut<'a, 'gm, V>>
    where
        K: TryFromScm<'gm> + ToScm<'gm> + 'gm,
        V: TryFromScm<'gm> + 'gm,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        let handle = unsafe { E::GET_HANDLE(self.scm.as_ptr(), key.to_scm(guile).as_ptr()) };
        if Pair::<K, V>::predicate(&Scm::from_ptr(handle, guile), guile) {
            Some(unsafe { RefMut::new_unchecked(scm_cdr(handle)) })
        } else {
            None
        }
    }
    pub fn insert(&mut self, key: K, val: V)
    where
        K: ToScm<'gm>,
        V: ToScm<'gm>,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        unsafe {
            E::SET(
                self.scm.as_ptr(),
                key.to_scm(guile).as_ptr(),
                val.to_scm(guile).as_ptr(),
            );
        }
    }
    pub fn remove(&mut self, key: K) -> Option<V>
    where
        K: ToScm<'gm> + TryFromScm<'gm> + 'gm,
        V: TryFromScm<'gm> + 'gm,
    {
        let guile = unsafe { Guile::new_unchecked_ref() };
        Pair::<K, V>::try_from_scm(
            Scm::from_ptr(
                unsafe { E::REMOVE(self.scm.as_ptr(), key.to_scm(guile).as_ptr()) },
                guile,
            ),
            guile,
        )
        .map(Pair::into_tuple)
        .map(|(_, r)| r)
        .ok()
    }
}
unsafe impl<K, V, E> ReprScm for HashMap<'_, K, V, E> where E: ScmPartialEq {}
impl<'gm, K, V, E> ToScm<'gm> for HashMap<'gm, K, V, E>
where
    E: ScmPartialEq,
{
    fn to_scm(self, _: &'gm Guile) -> Scm<'gm> {
        self.scm
    }
}
impl<'gm, K, V, E> TryFromScm<'gm> for HashMap<'gm, K, V, E>
where
    K: TryFromScm<'gm>,
    V: TryFromScm<'gm>,
    E: ScmPartialEq,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "(hash-map {} {})",
            K::type_name().display(),
            V::type_name().display()
        ))
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(c"hash-map"))
    }

    fn predicate(hm: &Scm<'gm>, guile: &'gm Guile) -> bool {
        Scm::from_ptr(unsafe { scm_hash_table_p(hm.as_ptr()) }, guile).is_true() && {
            let callback = unsafe {
                scm_c_make_gsubr(
                    c"hash-map-fold-callback".as_ptr(),
                    3,
                    0,
                    0,
                    hash_map_fold_callback::<'gm, K, V> as *mut c_void,
                )
            };
            Scm::from_ptr(
                unsafe { scm_hash_fold(callback, SCM_BOOL_F, hm.as_ptr()) },
                guile,
            )
            .is_true()
        }
    }

    unsafe fn from_scm_unchecked(scm: Scm<'gm>, _: &'gm Guile) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}

extern "C" fn hash_map_fold_callback<'gm, K, V>(key: SCM, val: SCM, accum: SCM) -> SCM
where
    K: TryFromScm<'gm>,
    V: TryFromScm<'gm>,
{
    let guile = unsafe { Guile::new_unchecked_ref() };
    if Scm::from_ptr(accum, guile).is_false() {
        false
    } else {
        let [key, val] = [key, val].map(|ptr| Scm::from_ptr(ptr, guile));
        K::predicate(&key, guile) && V::predicate(&val, guile)
    }
    .to_scm(guile)
    .as_ptr()
}
