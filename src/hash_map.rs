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

#![expect(private_bounds)]
#![expect(private_interfaces)]

use {
    crate::{
        Api, Scm, ScmTy,
        sys::{
            SCM, SCM_UNDEFINED, scm_hash_create_handle_x, scm_hash_get_handle, scm_hash_remove_x,
            scm_hash_set_x, scm_hashq_create_handle_x, scm_hashq_get_handle, scm_hashq_remove_x,
            scm_hashq_set_x, scm_hashv_create_handle_x, scm_hashv_get_handle, scm_hashv_remove_x,
            scm_hashv_set_x, scm_make_hash_table,
        },
    },
    bstr::BStr,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        marker::PhantomData,
    },
};

trait ScmPartialEq {
    const HASH_SET_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM;
    const HASH_REMOVE_X: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM;
    const HASH_GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM;
    const HASH_CREATE_HANDLE_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM;
}

struct Eq;
impl ScmPartialEq for Eq {
    const HASH_SET_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        scm_hashq_set_x;
    const HASH_REMOVE_X: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM = scm_hashq_remove_x;
    const HASH_GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        scm_hashq_get_handle;
    const HASH_CREATE_HANDLE_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        scm_hashq_create_handle_x;
}
pub type HashMapQ<'id, K, V> = InnerHashMap<'id, K, V, Eq>;

struct Eqv;
impl ScmPartialEq for Eqv {
    const HASH_SET_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        scm_hashv_set_x;
    const HASH_REMOVE_X: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM = scm_hashv_remove_x;
    const HASH_GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        scm_hashv_get_handle;
    const HASH_CREATE_HANDLE_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        scm_hashv_create_handle_x;
}
pub type HashMapV<'id, K, V> = InnerHashMap<'id, K, V, Eqv>;

struct Equal;
impl ScmPartialEq for Equal {
    const HASH_SET_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _val: SCM) -> SCM =
        scm_hash_set_x;
    const HASH_REMOVE_X: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM = scm_hash_remove_x;
    const HASH_GET_HANDLE: unsafe extern "C" fn(_table: SCM, _key: SCM) -> SCM =
        scm_hash_get_handle;
    const HASH_CREATE_HANDLE_X: unsafe extern "C" fn(_table: SCM, _key: SCM, _init: SCM) -> SCM =
        scm_hash_create_handle_x;
}
pub type HashMap<'id, K, V> = InnerHashMap<'id, K, V, Equal>;

#[derive(Debug)]
#[repr(transparent)]
pub struct InnerHashMap<'id, K, V, C>
where
    K: ScmTy<'id>,
    V: ScmTy<'id>,
    C: ScmPartialEq,
{
    scm: Scm<'id>,
    _marker: PhantomData<(K, V, C)>,
}
impl<'id, K, V, C> InnerHashMap<'id, K, V, C>
where
    K: ScmTy<'id>,
    V: ScmTy<'id>,
    C: ScmPartialEq,
{
    pub fn new(_: &'id Api) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr(scm_make_hash_table(SCM_UNDEFINED)) },
            _marker: PhantomData,
        }
    }
    pub fn with_capacity(api: &'id Api, n: usize) -> Self {
        Self {
            scm: unsafe { Scm::from_ptr(scm_make_hash_table(api.make(n).as_ptr())) },
            _marker: PhantomData,
        }
    }
}

impl<'id, K, V, C> ScmTy<'id> for InnerHashMap<'id, K, V, C>
where
    K: ScmTy<'id>,
    V: ScmTy<'id>,
    C: ScmPartialEq,
{
    fn type_name() -> Cow<'static, CStr> {
        CString::new(format!(
            "(hash-map {} {})",
            BStr::new(K::type_name().as_ref().to_bytes()),
            BStr::new(V::type_name().as_ref().to_bytes())
        ))
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(c"hash-map"))
    }
    fn construct(self) -> Scm<'id> {
        self.scm
    }
    fn predicate(_: &Api, _scm: &Scm) -> bool {
        todo!()
    }
    unsafe fn get_unchecked(_: &Api, scm: Scm<'id>) -> Self {
        Self {
            scm,
            _marker: PhantomData,
        }
    }
}
