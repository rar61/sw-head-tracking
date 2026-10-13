use std::{env, ffi::c_void, iter, mem, os::windows::ffi::OsStrExt, path::Path};

use windows::core::{s, w, PCWSTR};
use windows::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
use windows::Win32::Foundation::{HANDLE, HMODULE, ERROR_MOD_NOT_FOUND, MAX_PATH, WAIT_FAILED, CloseHandle, FreeLibrary};
use windows::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory,
    LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx},
    ProcessStatus::{EnumProcessModules, GetModuleBaseNameW},
    Threading::{LPTHREAD_START_ROUTINE, INFINITE, PROCESS_ALL_ACCESS, CreateRemoteThread, GetExitCodeThread, OpenProcess, WaitForSingleObject}
};
use pattern::{Pattern, Process};

mod pattern;

const TARGET: Pattern<'_> = pattern!("4c 8b dc 55 57 49 8d ab f8 fc ff ff");

fn main() {
    let args: Vec<_> = env::args_os().collect();
    let pid = args[1].to_str().unwrap().parse().unwrap();

    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, false, pid).unwrap();
        let process = Process::new(process_handle);

        let results = process.scan(TARGET, 128 << 10).unwrap();

        if results.len() > 1 { panic!() }

        let addr = *results.first().unwrap();

        inject_dll(process_handle, "sw_head_tracking.dll", addr).unwrap();
    }
}

fn inject_dll(process_handle: HANDLE, dll_path: impl AsRef<Path>, targer_addr: usize) -> Result<(), windows::core::Error> {
    let dll_path = dll_path.as_ref();
    let path_buf: Vec<u16> = dll_path.as_os_str().encode_wide().chain(iter::once(0)).collect();
    unsafe {
        let kernel32_handle = GetModuleHandleW(w!("kernel32.dll"))?;
        let load_library = GetProcAddress(kernel32_handle, s!("LoadLibraryW")).ok_or(windows::core::Error::from_thread())?;

        call_remote(process_handle, Some(mem::transmute(load_library)), path_buf.as_slice())?;
        
        let dll_handle = LoadLibraryW(PCWSTR::from_raw(path_buf.as_ptr()))?;
        let Some(initialize) = GetProcAddress(dll_handle, s!("initialize")) else {
            return Err(windows::core::Error::from_thread());
        };
        FreeLibrary(dll_handle)?;

        let offset = initialize as usize - dll_handle.0 as usize;

        let remote_module = get_remote_module(process_handle, w!("sw_head_tracking.dll"))?;
        let remote_initialize = remote_module.0.byte_add(offset);

        call_remote(process_handle, Some(mem::transmute(remote_initialize)), &targer_addr)?;
    }

    Ok(())
}

unsafe fn call_remote_raw(process_handle: HANDLE, remote_proc_address: LPTHREAD_START_ROUTINE, parameter: Option<*const c_void>) -> Result<u32, windows::core::Error> {
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

unsafe fn call_remote<T: ?Sized>(process_handle: HANDLE, remote_proc_address: LPTHREAD_START_ROUTINE, parameter: &T) -> Result<u32, windows::core::Error> {
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

unsafe fn get_remote_module(process_handle: HANDLE, module_name: PCWSTR) -> Result<HMODULE, windows::core::Error> {
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