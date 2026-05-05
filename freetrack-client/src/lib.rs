use std::ffi::c_void;

use windows::core::{s, PCSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::System::Memory::{CreateFileMappingA, MapViewOfFile, UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS, FILE_MAP_ALL_ACCESS, PAGE_READWRITE};
use windows::Win32::System::Threading::{CreateMutexA, ReleaseMutex, WaitForSingleObject};

const FT_MM_DATA: PCSTR = s!("FT_SharedMem");
const FREETRACK_MUTEX: PCSTR = s!("FT_Mutext");
const MUTEX_WAIT_TIME: u32 = 16;

pub struct FTClient {
    map: HANDLE,
    view: *mut FTData,
    mutex: HANDLE
}

unsafe impl Send for FTClient {}
unsafe impl Sync for FTClient {}

impl FTClient {
    pub fn new() -> Result<Self, windows::core::Error> {
        unsafe {
            let map = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                size_of::<FTData>() as u32,
                FT_MM_DATA
            )?;
            let view = {
                let view = MapViewOfFile(
                    map,
                    FILE_MAP_ALL_ACCESS,
                    0,
                    0,
                    size_of::<FTData>()
                ).Value;
                if view.is_null() {
                    Err(windows::core::Error::from_thread())
                } else {
                    Ok(view.cast())
                }
            }?;
            let mutex = CreateMutexA(
                None,
                false,
                FREETRACK_MUTEX
            )?;

            Ok(Self { map, view, mutex })
        }
    }

    pub fn get_data(&self) -> Option<FTData> {
        unsafe {
            if WaitForSingleObject(self.mutex, MUTEX_WAIT_TIME) == WAIT_OBJECT_0 {
                let data = self.view.read_unaligned();
                ReleaseMutex(self.mutex).expect("failed to release a mutex");
                Some(data)
            } else {
                None  
            }
        }
    }

    pub fn get_slim_data(&self) -> Option<FTSlimData> {
        unsafe {
            if WaitForSingleObject(self.mutex, MUTEX_WAIT_TIME) == WAIT_OBJECT_0 {
                let data = (self.view as *mut FTSlimData).read_unaligned();
                ReleaseMutex(self.mutex).expect("failed to release a mutex");
                Some(data)
            } else {
                None
            }
        }
    }
}

impl Drop for FTClient {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.view as *mut c_void }).unwrap();
            CloseHandle(self.map).unwrap();
            CloseHandle(self.mutex).unwrap();
        }
    }
}

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct FTData {
    pub data_id: u32,
    pub cam_width: i32,
    pub cam_height: i32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub raw_yaw: f32,
    pub raw_pitch: f32,
    pub raw_roll: f32,
    pub raw_x: f32,
    pub raw_y: f32,
    pub raw_z: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub x3: f32,
    pub y3: f32,
    pub x4: f32,
    pub y4: f32
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FTSlimData {
    pub data_id: u32,
    pub cam_width: i32,
    pub cam_height: i32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32
}
