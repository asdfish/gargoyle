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

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct scm_unused_struct {
    pub scm_unused_field: c_char,
}

pub type SCM = *mut scm_unused_struct;

unsafe extern "C" {
    pub fn malloc(_: usize) -> *mut c_void;
    pub fn free(_: *mut c_void);

    pub static GARGOYLE_REEXPORTS_SCM_BOOL_T: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_BOOL_F: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_UNDEFINED: SCM;

    pub fn scm_with_guile(
        _: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
        _: *mut c_void,
    ) -> *mut c_void;

    pub fn scm_from_utf8_stringn(_: *const c_char, _: usize) -> SCM;
    pub fn scm_to_utf8_stringn(_: SCM, _: *mut usize) -> *mut c_char;

    pub fn scm_to_bool(_: SCM) -> c_int;

    pub fn scm_from_uint32(_: u32) -> SCM;
    pub fn scm_to_uint32(_: SCM) -> u32;

    pub fn scm_integer_to_char(_: SCM) -> SCM;
    pub fn scm_char_to_integer(_: SCM) -> SCM;

    pub fn scm_gc_protect_object(_: SCM) -> SCM;
    pub fn scm_gc_unprotect_object(_: SCM) -> SCM;
}
