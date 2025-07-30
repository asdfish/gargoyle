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

#![expect(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_void};

#[repr(C)]
pub struct scm_unused_struct {
    pub scm_unused_field: c_char,
}

pub type SCM = *mut scm_unused_struct;

pub type scm_t_dynwind_flags = c_int;
pub type scm_t_wind_flags = c_int;

pub type scm_t_thunk = Option<unsafe extern "C" fn(*mut c_void) -> SCM>;
pub type scm_t_catch_body = scm_t_thunk;
pub type scm_t_catch_handler = Option<unsafe extern "C" fn(*mut c_void, SCM, SCM) -> SCM>;

#[derive(Default)]
#[repr(C)]
pub struct scm_t_array_dim {
    lbnd: isize,
    ubnd: isize,
    inc: isize,
}
#[derive(Default)]
#[repr(C)]
pub enum scm_t_array_element_type {
    #[default]
    SCM_ARRAY_ELEMENT_TYPE_SCM = 0,
    SCM_ARRAY_ELEMENT_TYPE_CHAR = 1,
    SCM_ARRAY_ELEMENT_TYPE_BIT = 2,
    SCM_ARRAY_ELEMENT_TYPE_VU8 = 3,
    SCM_ARRAY_ELEMENT_TYPE_U8 = 4,
    SCM_ARRAY_ELEMENT_TYPE_S8 = 5,
    SCM_ARRAY_ELEMENT_TYPE_U16 = 6,
    SCM_ARRAY_ELEMENT_TYPE_S16 = 7,
    SCM_ARRAY_ELEMENT_TYPE_U32 = 8,
    SCM_ARRAY_ELEMENT_TYPE_S32 = 9,
    SCM_ARRAY_ELEMENT_TYPE_U64 = 10,
    SCM_ARRAY_ELEMENT_TYPE_S64 = 11,
    SCM_ARRAY_ELEMENT_TYPE_F32 = 12,
    SCM_ARRAY_ELEMENT_TYPE_F64 = 13,
    SCM_ARRAY_ELEMENT_TYPE_C32 = 14,
    SCM_ARRAY_ELEMENT_TYPE_C64 = 15,
}

// pub const SCM_ARRAY_ELEMENT_TYPE_LAST: scm_t_array_element_type =
//     scm_t_array_element_type::SCM_ARRAY_ELEMENT_TYPE_C64;

pub type scm_t_vector_ref = Option<extern "C" fn(_: SCM, _: usize) -> SCM>;
pub type scm_t_vector_set = Option<extern "C" fn(_: SCM, _: usize, SCM)>;
#[derive(Default)]
#[repr(C)]
pub struct scm_t_array_handle {
    array: SCM,

    base: usize,
    ndims: usize,
    dims: *mut scm_t_array_dim,
    dim0: scm_t_array_dim,
    element_type: scm_t_array_element_type,
    elements: *const c_void,
    writable_elements: *mut c_void,

    vector: SCM,
    vref: scm_t_vector_ref,
    vset: scm_t_vector_set,
}

unsafe extern "C" {
    pub static GARGOYLE_REEXPORTS_SCM_BOOL_T: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_BOOL_F: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_EOL: SCM;
    pub static GARGOYLE_REEXPORTS_SCM_UNDEFINED: SCM;

    pub static GARGOYLE_REEXPORTS_SCM_F_DYNWIND_REWINDABLE: c_int;
    pub static GARGOYLE_REEXPORTS_SCM_F_WIND_EXPLICITLY: c_int;

    pub fn GARGOYLE_REEXPORTS_SCM_UNBNDP(_: SCM) -> bool;

    pub fn scm_with_guile(
        _func: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
        _data: *mut c_void,
    ) -> *mut c_void;
    pub fn scm_shell(_argc: c_int, _argv: *const *const c_char);

    pub fn scm_from_utf8_stringn(_: *const c_char, _: usize) -> SCM;
    pub fn scm_to_utf8_stringn(_: SCM, _: *mut usize) -> *mut c_char;

    pub fn scm_to_bool(_: SCM) -> bool;
    pub fn scm_not(_: SCM) -> SCM;

    pub fn scm_integer_to_char(_: SCM) -> SCM;
    pub fn scm_char_to_integer(_: SCM) -> SCM;
    pub fn scm_char_p(_: SCM) -> SCM;

    pub fn scm_char_set_p(_obj: SCM) -> SCM;
    pub fn scm_char_set_contains_p(_cs: SCM, _ch: SCM) -> SCM;
    pub fn scm_char_set_ref(_cs: SCM, _cursor: SCM) -> SCM;
    pub fn scm_char_set_cursor(_cs: SCM) -> SCM;
    pub fn scm_char_set_cursor_next(_cs: SCM, _cursor: SCM) -> SCM;

    pub fn scm_end_of_char_set_p(_cursor: SCM) -> SCM;

    pub fn scm_from_double(_: c_double) -> SCM;
    pub fn scm_from_int8(_: i8) -> SCM;
    pub fn scm_from_uint8(_: u8) -> SCM;
    pub fn scm_from_int16(_: i16) -> SCM;
    pub fn scm_from_uint16(_: u16) -> SCM;
    pub fn scm_from_int32(_: i32) -> SCM;
    pub fn scm_from_uint32(_: u32) -> SCM;
    pub fn scm_from_int64(_: i64) -> SCM;
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
    pub fn scm_to_int64(_: SCM) -> i64;
    pub fn scm_to_uint64(_: SCM) -> u64;
    pub fn gargoyle_reexports_scm_to_intptr_t(_: SCM) -> isize;
    pub fn gargoyle_reexports_scm_to_uintptr_t(_: SCM) -> usize;

    pub fn scm_car(_pair: SCM) -> SCM;
    pub fn scm_cdr(_pair: SCM) -> SCM;
    pub fn scm_cons(_x: SCM, _y: SCM) -> SCM;
    pub fn scm_length(_lst: SCM) -> SCM;

    pub fn scm_list_p(_x: SCM) -> SCM;
    pub fn scm_null_p(_x: SCM) -> SCM;

    pub fn scm_list_to_char_set(_list: SCM, _base_cs: SCM) -> SCM;

    pub fn scm_vector_p(_obj: SCM) -> SCM;
    pub fn scm_u8vector_p(_obj: SCM) -> SCM;
    pub fn scm_s8vector_p(_obj: SCM) -> SCM;
    pub fn scm_u16vector_p(_obj: SCM) -> SCM;
    pub fn scm_s16vector_p(_obj: SCM) -> SCM;
    pub fn scm_u32vector_p(_obj: SCM) -> SCM;
    pub fn scm_s32vector_p(_obj: SCM) -> SCM;
    pub fn scm_u64vector_p(_obj: SCM) -> SCM;
    pub fn scm_s64vector_p(_obj: SCM) -> SCM;
    pub fn scm_f32vector_p(_obj: SCM) -> SCM;
    pub fn scm_f64vector_p(_obj: SCM) -> SCM;
    pub fn scm_c32vector_p(_obj: SCM) -> SCM;
    pub fn scm_c64vector_p(_obj: SCM) -> SCM;

    pub fn scm_vector(_l: SCM) -> SCM;
    pub fn scm_list_to_u8vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_s8vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_u16vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_s16vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_u32vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_s32vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_u64vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_s64vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_f32vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_f64vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_c32vector(_lst: SCM) -> SCM;
    pub fn scm_list_to_c64vector(_lst: SCM) -> SCM;

    pub fn scm_vector_to_list(_v: SCM) -> SCM;
    pub fn scm_u8vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_s8vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_u16vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_s16vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_u32vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_s32vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_u64vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_s64vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_f32vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_f64vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_c32vector_to_list(_vec: SCM) -> SCM;
    pub fn scm_c64vector_to_list(_vec: SCM) -> SCM;

    pub fn scm_vector_elements(
        _array: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const SCM;
    pub fn scm_u8vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const u8;
    pub fn scm_s8vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const i8;
    pub fn scm_u16vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const u16;
    pub fn scm_s16vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const i16;
    pub fn scm_u32vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const u32;
    pub fn scm_s32vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const i32;
    pub fn scm_u64vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const u64;
    pub fn scm_s64vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const i64;
    pub fn scm_f32vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const f32;
    pub fn scm_f64vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const f64;
    pub fn scm_c32vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const f32;
    pub fn scm_c64vector_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *const f64;

    pub fn scm_vector_writable_elements(
        _array: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut SCM;
    pub fn scm_u8vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut u8;
    pub fn scm_s8vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut i8;
    pub fn scm_u16vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut u16;
    pub fn scm_s16vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut i16;
    pub fn scm_u32vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut u32;
    pub fn scm_s32vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut i32;
    pub fn scm_u64vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut u64;
    pub fn scm_s64vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut i64;
    pub fn scm_f32vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut f32;
    pub fn scm_f64vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut f64;
    pub fn scm_c32vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut f32;
    pub fn scm_c64vector_writable_elements(
        _vec: SCM,
        _handle: *mut scm_t_array_handle,
        _lenp: *mut usize,
        _incp: *mut isize,
    ) -> *mut f64;

    pub fn scm_array_handle_release(_handle: *mut scm_t_array_handle);

    pub fn scm_is_signed_integer(_: SCM, _: isize, _: isize) -> bool;
    pub fn scm_is_unsigned_integer(_: SCM, _: usize, _: usize) -> bool;

    pub fn scm_is_bool(_val: SCM) -> bool;
    pub fn scm_is_complex(_val: SCM) -> bool;
    pub fn scm_is_exact(_val: SCM) -> bool;

    pub fn scm_is_inexact(_val: SCM) -> bool;
    pub fn scm_is_rational(_val: SCM) -> bool;
    pub fn scm_is_string(_val: SCM) -> bool;
    pub fn gargoyle_reexports_scm_is_true(_val: SCM) -> bool;

    pub fn scm_is_exact_integer(_val: SCM) -> bool;
    pub fn scm_exact_to_inexact(_z: SCM) -> SCM;
    pub fn scm_inexact_to_exact(_z: SCM) -> SCM;

    pub fn scm_sum(_z1: SCM, _z2: SCM) -> SCM;
    pub fn scm_difference(_z1: SCM, _z2: SCM) -> SCM;
    pub fn scm_divide(_z1: SCM, _z2: SCM) -> SCM;
    pub fn scm_remainder(_n: SCM, _d: SCM) -> SCM;
    pub fn scm_product(_z1: SCM, _z2: SCM) -> SCM;

    pub fn scm_logand(_n1: SCM, _n2: SCM) -> SCM;
    pub fn scm_logior(_n1: SCM, _n2: SCM) -> SCM;
    pub fn scm_logxor(_n1: SCM, _n2: SCM) -> SCM;

    pub fn scm_nan() -> SCM;
    pub fn scm_inf() -> SCM;

    pub fn scm_rationalize(_x: SCM, _eps: SCM) -> SCM;
    pub fn scm_numerator(_val: SCM) -> SCM;
    pub fn scm_denominator(_val: SCM) -> SCM;

    pub fn scm_real_part(_z: SCM) -> SCM;
    pub fn scm_imag_part(_z: SCM) -> SCM;

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
    pub fn scm_misc_error(_subr: *const c_char, _msg: *const c_char, _args: SCM);

    pub fn scm_c_make_gsubr(_: *const c_char, _: c_int, _: c_int, _: c_int, _: *mut c_void) -> SCM;
    pub fn scm_c_define_gsubr(
        _: *const c_char,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut c_void,
    ) -> SCM;

    pub fn scm_c_eval_string(_: *const c_char) -> SCM;
    pub fn scm_c_primitive_load(_: *const c_char) -> SCM;

    pub fn scm_c_string_length(_: SCM) -> usize;

    pub fn scm_string_to_char_set(_str: SCM, _base_cs: SCM) -> SCM;

    pub fn scm_call_n(_: SCM, _: *mut SCM, _: usize) -> SCM;

    pub fn scm_open_output_string() -> SCM;
    pub fn scm_strport_to_string(_: SCM) -> SCM;

    pub fn scm_close_port(_: SCM) -> SCM;
    pub fn scm_write(_: SCM, _: SCM) -> SCM;

    pub fn scm_dynwind_begin(_: scm_t_dynwind_flags);
    pub fn scm_dynwind_unwind_handler(
        _: Option<unsafe extern "C" fn(_: *mut c_void)>,
        _: *mut c_void,
        _: scm_t_wind_flags,
    );
    pub fn scm_dynwind_end();

    pub fn scm_internal_catch(
        _tag: SCM,
        _body: scm_t_catch_body,
        _body_data: *mut c_void,
        _handler: scm_t_catch_handler,
        _handler_data: *mut c_void,
    ) -> SCM;
}

pub use GARGOYLE_REEXPORTS_SCM_BOOL_F as SCM_BOOL_F;
pub use GARGOYLE_REEXPORTS_SCM_BOOL_T as SCM_BOOL_T;
pub use GARGOYLE_REEXPORTS_SCM_EOL as SCM_EOL;
pub use GARGOYLE_REEXPORTS_SCM_F_DYNWIND_REWINDABLE as SCM_F_DYNWIND_REWINDABLE;
pub use GARGOYLE_REEXPORTS_SCM_F_WIND_EXPLICITLY as SCM_F_WIND_EXPLICITLY;
pub use GARGOYLE_REEXPORTS_SCM_UNBNDP as SCM_UNBNDP;
pub use GARGOYLE_REEXPORTS_SCM_UNDEFINED as SCM_UNDEFINED;
pub use gargoyle_reexports_scm_from_intptr_t as scm_from_intptr_t;
pub use gargoyle_reexports_scm_from_uintptr_t as scm_from_uintptr_t;
pub use gargoyle_reexports_scm_is_true as scm_is_true;
pub use gargoyle_reexports_scm_to_intptr_t as scm_to_intptr_t;
pub use gargoyle_reexports_scm_to_uintptr_t as scm_to_uintptr_t;
