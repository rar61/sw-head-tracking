use freetrack_client::FTClient;
use minhook_sys::{MH_CreateHook, MH_EnableHook, MH_Initialize};

use std::{ffi::c_void, mem, ptr::{self, null_mut}, sync::{OnceLock, atomic::{AtomicBool, Ordering}}};

use crate::dll_config::DllConfig;

mod dll_config;

static ORIGINAL: OnceLock<fn(*mut c_void, *mut c_void, f32, i32)> = OnceLock::new();
static CLIENT: OnceLock<FTClient> = OnceLock::new();
static ENABLE_ROLL: AtomicBool = AtomicBool::new(false);

extern "C" fn detour(data: *mut c_void, data2: *mut c_void, time: f32, var2: i32) {
    if let Some(o) = ORIGINAL.get() {
        if unsafe { is_seated(data) & !(is_in_scope(data) | is_third_person(data)) } {
            if let Some(c) = CLIENT.get() {
                if let Some(d)= c.get_slim_data() {
                    unsafe {
                        set_pitch(data, d.pitch);
                        set_yaw(data, -d.yaw);
                        if ENABLE_ROLL.load(Ordering::Relaxed) { set_roll(data, d.roll) };
                    }
                }
            }
        }
        o(data, data2, time, var2)
    }
}

unsafe fn is_seated(transform: *mut c_void) -> bool {
    let value = unsafe { (transform.byte_add(0xe20) as *mut u8).read() };
    value != 0
}

unsafe fn is_in_scope(transform: *mut c_void) -> bool {
    let value = unsafe { (transform.byte_add(0x1a7) as *mut u8).read() };
    value != 0
}

unsafe fn is_third_person(transform: *mut c_void) -> bool {
    let value = unsafe { (transform.byte_add(0x1a0) as *mut u8).read() };
    value != 0
}

unsafe fn set_pitch(transform: *mut c_void, pitch: f32) {
    unsafe {
        let pitch_ptr = transform.byte_add(0x2a4) as *mut f32;
        ptr::write_unaligned(pitch_ptr, pitch);
    }
}

unsafe fn set_yaw(transform: *mut c_void, yaw: f32) {
    unsafe {
        let yaw_ptr = transform.byte_add(0x2a0) as *mut f32;
        ptr::write_unaligned(yaw_ptr, yaw);
    }
}

unsafe fn set_roll(transform: *mut c_void, roll: f32) {
    unsafe {
        let roll_ptr = transform.byte_add(0x2c0) as *mut f32;
        ptr::write_unaligned(roll_ptr, roll);
    }
}

#[unsafe(no_mangle)]
extern "system" fn initialize(parameter: *mut c_void) -> u32 {
    let ft_client = FTClient::new().unwrap();
    if CLIENT.set(ft_client).is_err() { panic!("freetrack client already initialized") };

    unsafe {
        if MH_Initialize() != 0 {
            panic!();
        }

        let config = parameter.cast::<DllConfig>().as_mut().unwrap();

        ENABLE_ROLL.store(config.enable_roll, Ordering::Relaxed);

        let mut original = null_mut();
        if MH_CreateHook(config.target_address, detour as *mut c_void, &mut original) != 0 {
            panic!();
        }
        
        if !original.is_null() {
            ORIGINAL.set(mem::transmute(original)).unwrap();
        }
        
        if MH_EnableHook(null_mut()) != 0 {
            panic!();
        }
    }

    0
}