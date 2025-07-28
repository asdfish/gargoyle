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

use std::ffi::{c_char, c_double, c_int, c_void};

#[repr(C)]
pub struct scm_unused_struct {
    pub scm_unused_field: c_char,
}

pub type SCM = *mut scm_unused_struct;

unsafe extern "C" {
    pub static GARGOYLE_REEXPORTS_SCM_BOOL_T: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_BOOL_F: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_UNDEFINED: SCM;

    pub fn scm_with_guile(
        _: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
        _: *mut c_void,
    ) -> *mut c_void;

    pub fn scm_from_utf8_stringn(_: *const c_char, _: usize) -> SCM;
    pub fn scm_to_utf8_stringn(_: SCM, _: *mut usize) -> *mut c_char;

    pub fn scm_to_bool(_: SCM) -> bool;
    pub fn scm_not(_: SCM) -> SCM;

    pub fn scm_integer_to_char(_: SCM) -> SCM;
    pub fn scm_char_to_integer(_: SCM) -> SCM;
    pub fn scm_char_p(_: SCM) -> SCM;

    pub fn scm_from_double(_: c_double) -> SCM;
    pub fn scm_from_int8(_: i8) -> SCM;
    pub fn scm_from_uint8(_: u8) -> SCM;
    pub fn scm_from_int16(_: i16) -> SCM;
    pub fn scm_from_uint16(_: u16) -> SCM;
    pub fn scm_from_int32(_: i32) -> SCM;
    pub fn scm_from_uint32(_: u32) -> SCM;
    #[cfg(target_pointer_width = "64")]
    pub fn scm_from_int64(_: i64) -> SCM;
    #[cfg(target_pointer_width = "64")]
    pub fn scm_from_uint64(_: u64) -> SCM;
    pub fn gargoyle_reexports_scm_from_intptr_t(_: isize) -> SCM;
    pub fn gargoyle_reexports_scm_from_uintptr_t(_: usize) -> SCM;

    pub fn scm_to_double(_: SCM) -> c_double;
    pub fn scm_to_int8(_: SCM) -> i8;
    pub fn scm_to_uint8(_: SCM) -> u8;
    pub fn scm_to_int16(_: SCM) -> i16;
    pub fn scm_to_uint16(_: SCM) -> u16;
    pub fn scm_to_int32(_: SCM) -> i32;
    pub fn scm_to_uint32(_: SCM) -> u32;
    #[cfg(target_pointer_width = "64")]
    pub fn scm_to_int64(_: SCM) -> i64;
    #[cfg(target_pointer_width = "64")]
    pub fn scm_to_uint64(_: SCM) -> u64;
    pub fn gargoyle_reexports_scm_to_intptr_t(_: SCM) -> isize;
    pub fn gargoyle_reexports_scm_to_uintptr_t(_: SCM) -> usize;

    pub fn scm_is_signed_integer(_: SCM, _: isize, _: isize) -> bool;
    pub fn scm_is_unsigned_integer(_: SCM, _: usize, _: usize) -> bool;

    pub fn scm_is_bool(_: SCM) -> bool;
    pub fn scm_is_string(_: SCM) -> bool;
    pub fn gargoyle_reexports_scm_is_true(_: SCM) -> bool;

    pub fn scm_gc_protect_object(_: SCM) -> SCM;
    pub fn scm_gc_unprotect_object(_: SCM) -> SCM;

    pub fn scm_eq_p(_: SCM, _: SCM) -> SCM;
    pub fn scm_eqv_p(_: SCM, _: SCM) -> SCM;
    pub fn scm_equal_p(_: SCM, _: SCM) -> SCM;

    pub fn scm_is_number(_: SCM) -> bool;
    pub fn scm_is_real(_: SCM) -> bool;

    pub fn scm_num_eq_p(_: SCM, _: SCM) -> SCM;
    pub fn scm_less_p(_: SCM, _: SCM) -> SCM;
    pub fn scm_gr_p(_: SCM, _: SCM) -> SCM;

    pub fn scm_wrong_type_arg_msg(_: *const c_char, _: c_int, _: SCM, _: *const c_char);

    pub fn scm_c_define_gsubr(
        _: *const c_char,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut c_void,
    ) -> SCM;
    pub fn scm_c_eval_string(_: *const c_char) -> SCM;

    pub fn GARGOYLE_REEXPORTS_SCM_UNBNDP(_: SCM) -> bool;
}

pub use GARGOYLE_REEXPORTS_SCM_BOOL_F as SCM_BOOL_F;
pub use GARGOYLE_REEXPORTS_SCM_BOOL_T as SCM_BOOL_T;
pub use GARGOYLE_REEXPORTS_SCM_UNBNDP as SCM_UNBNDP;
pub use GARGOYLE_REEXPORTS_SCM_UNDEFINED as SCM_UNDEFINED;
pub use gargoyle_reexports_scm_from_intptr_t as scm_from_intptr_t;
pub use gargoyle_reexports_scm_from_uintptr_t as scm_from_uintptr_t;
pub use gargoyle_reexports_scm_is_true as scm_is_true;
pub use gargoyle_reexports_scm_to_intptr_t as scm_to_intptr_t;
pub use gargoyle_reexports_scm_to_uintptr_t as scm_to_uintptr_t;
