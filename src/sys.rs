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

    pub fn scm_gc_protect_object(_: SCM) -> SCM;
    pub fn scm_gc_unprotect_object(_: SCM) -> SCM;
}
