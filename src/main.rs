#![windows_subsystem = "windows"]

use std::{ffi::c_void, iter, mem, os::windows::ffi::OsStrExt, path::Path};

use windows::Win32::System::Threading::{CREATE_SUSPENDED, CreateProcessW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, ResumeThread, STARTUPINFOW};
use windows::core::{Owned, PCWSTR, s, w};
use windows::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
use windows::Win32::Foundation::{HANDLE, HMODULE, ERROR_MOD_NOT_FOUND, MAX_PATH, WAIT_FAILED, CloseHandle, FreeLibrary};
use windows::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory,
    LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx},
    ProcessStatus::{EnumProcessModules, GetModuleBaseNameW},
    Threading::{LPTHREAD_START_ROUTINE, INFINITE, CreateRemoteThread, GetExitCodeThread, WaitForSingleObject}
};

use crate::dll_config::DllConfig;
use crate::pattern::{Pattern, Process};
use crate::config::{CONFIG_PATH, Config};

mod config;
mod dll_config;
mod pattern;

type WinResult<T> = windows::core::Result<T>;

const TARGET: Pattern<'_> = pattern!("4c 8b dc 55 57 49 8d ab f8 fc ff ff");

fn main() {
    let mut config = Config::default();
    config.from_file(CONFIG_PATH);
    config.from_args();

    let (process_handle, thread_handle) = create_process(&config.exe_path, true).unwrap();

    unsafe {
        let process = Process::new(*process_handle);
        let results = process.scan(TARGET, 128 << 10).unwrap();
        if results.len() > 1 { panic!() }

        let target_address = *results.first().unwrap() as *mut c_void;

        let dll_config = DllConfig { target_address, enable_roll: config.enable_roll };

        inject_dll(*process_handle, config.dll_path, dll_config).unwrap();

        ResumeThread(*thread_handle);
    }
}

fn create_process(exe_path: impl AsRef<Path>, create_suspended: bool) -> WinResult<(Owned<HANDLE>, Owned<HANDLE>)> {
    let path_buf: Vec<u16> = exe_path.as_ref().as_os_str().encode_wide().chain(iter::once(0)).collect();
    let creation_flags = if create_suspended { CREATE_SUSPENDED } else { PROCESS_CREATION_FLAGS::default() };

    let mut startup_info = STARTUPINFOW::default();
    startup_info.cb = mem::size_of::<STARTUPINFOW>() as u32;
    let mut process_info = PROCESS_INFORMATION::default();

    unsafe {
        CreateProcessW(
            PCWSTR(path_buf.as_ptr()),
            None, None, None,
            false,
            creation_flags,
            None, None,
            &startup_info,
            &mut process_info
        )?;
    }

    let process_handle = unsafe { Owned::new(process_info.hProcess) };
    let thread_handle = unsafe { Owned::new(process_info.hThread) };        

    Ok((process_handle, thread_handle))
}

unsafe fn inject_dll(process_handle: HANDLE, dll_path: impl AsRef<Path>, config: DllConfig) -> WinResult<()> {
    let dll_path = dll_path.as_ref();
    let path_buf: Vec<u16> = dll_path.as_os_str().encode_wide().chain(iter::once(0)).collect();
    let dll_file_name = dll_path.file_name().unwrap_or_default();
    let name_buf: Vec<u16> = dll_file_name.encode_wide().chain(iter::once(0)).collect();
    unsafe {
        let kernel32_handle = GetModuleHandleW(w!("kernel32.dll"))?;
        let load_library = GetProcAddress(kernel32_handle, s!("LoadLibraryW")).ok_or(windows::core::Error::from_thread())?;

        call_remote(process_handle, Some(mem::transmute(load_library)), path_buf.as_slice())?;
        
        let dll_handle = LoadLibraryW(PCWSTR(path_buf.as_ptr()))?;
        let Some(initialize) = GetProcAddress(dll_handle, s!("initialize")) else {
            return Err(windows::core::Error::from_thread());
        };
        FreeLibrary(dll_handle)?;

        let offset = initialize as usize - dll_handle.0 as usize;

        let remote_module = get_remote_module(process_handle, PCWSTR(name_buf.as_ptr()))?;
        let remote_initialize = remote_module.0.byte_add(offset);

        call_remote(process_handle, Some(mem::transmute(remote_initialize)), &config)?;
    }

    Ok(())
}

unsafe fn call_remote_raw(process_handle: HANDLE, remote_proc_address: LPTHREAD_START_ROUTINE, parameter: Option<*const c_void>) -> WinResult<u32> {
    unsafe {
        let thread_handle = CreateRemoteThread(process_handle, None, 0, remote_proc_address, parameter, 0, None)?;
        if WaitForSingleObject(thread_handle, INFINITE) == WAIT_FAILED {
            return Err(windows::core::Error::from_thread());
        }
    
        let mut result = 0;
        GetExitCodeThread(thread_handle, &mut result)?;

        CloseHandle(thread_handle)?;

        Ok(result)
    }
}

unsafe fn call_remote<T: ?Sized>(process_handle: HANDLE, remote_proc_address: LPTHREAD_START_ROUTINE, parameter: &T) -> WinResult<u32> {
    unsafe {
        let ptr = parameter as *const T as *const c_void;
        let size = size_of_val(parameter);

        let remote_buf = VirtualAllocEx(process_handle, None, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        if remote_buf.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        WriteProcessMemory(process_handle, remote_buf, ptr, size, None)?;

        let result = call_remote_raw(process_handle, remote_proc_address, Some(remote_buf))?;

        VirtualFreeEx(process_handle, remote_buf, 0, MEM_RELEASE)?;
        
        Ok(result)
    }
}

unsafe fn get_remote_module(process_handle: HANDLE, module_name: PCWSTR) -> WinResult<HMODULE> {
    unsafe {
        let mut modules = vec![Default::default(); 128];
        let mut needed = 0;
        loop {
            let size = mem::size_of_val(modules.as_slice()) as u32;
            EnumProcessModules(process_handle, modules.as_mut_ptr(), size, &mut needed)?;

            if needed <= size { break }

            modules.resize(modules.len() * 2, Default::default());
        }

        let count = needed as usize / mem::size_of::<HMODULE>();
        modules.truncate(count);

        for module in modules {
            let mut name_buf = [0; MAX_PATH as usize];
            let len = GetModuleBaseNameW(process_handle, Some(module), &mut name_buf);
            if len == 0 {
                return Err(windows::core::Error::from_thread());
            }

            let module_base_name = &name_buf[..len as usize];

            let cmp = CompareStringOrdinal(module_name.as_wide(), module_base_name, true);
            if cmp == CSTR_EQUAL {
                return Ok(module);
            }
        }

        Err(windows::core::Error::from_hresult(ERROR_MOD_NOT_FOUND.into()))
    }
}