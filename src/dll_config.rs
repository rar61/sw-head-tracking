use std::ffi::c_void;

#[repr(C)]
pub struct DllConfig {
    pub target_address: *mut c_void,
    pub enable_roll: bool
}