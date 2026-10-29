#![windows_subsystem = "windows"]

use std::error::Error;
use std::ffi::{OsStr, c_void};
use std::io::{self, Write};
use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
use std::panic;
use std::process::ExitCode;
use std::{iter, mem, path::Path};

use windows::core::{Owned, PCWSTR, s, w};
use windows::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
use windows::Win32::Foundation::{ERROR_MOD_NOT_FOUND, HANDLE, HMODULE, INVALID_HANDLE_VALUE, MAX_PATH, WAIT_FAILED};
use windows::Win32::System::{
    Console::{ATTACH_PARENT_PROCESS, AttachConsole},
    Diagnostics::Debug::WriteProcessMemory,
    LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx},
    ProcessStatus::{EnumProcessModules, GetModuleBaseNameW},
    Threading::{
        LPTHREAD_START_ROUTINE, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW, CREATE_SUSPENDED,
        INFINITE, CreateProcessW, CreateRemoteThread, GetExitCodeThread, ResumeThread, WaitForSingleObject
    }
};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW};

use crate::config::Command::Run;
use crate::dll_config::DllConfig;
use crate::pattern::{Pattern, Process};
use crate::config::{Config, print_help};

mod config;
mod dll_config;
mod pattern;

type WinResult<T> = windows::core::Result<T>;

const TARGET: Pattern<'_> = pattern!("4c 8b dc 55 57 49 8d ab f8 fc ff ff");

fn main() -> ExitCode {
    panic::set_hook(Box::new(|info| report_error(&info.to_string())));

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&e.to_string());
            ExitCode::FAILURE
        },
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let Run(config) = Config::load()? else {
        print_help();
        return Ok(());
    };

    let (process_handle, thread_handle) = create_process(&config.exe_path, true)?;

    unsafe {
        let process = Process::new(&process_handle);
        let results = process.scan(TARGET, 128 << 10)?;

        if results.len() > 1 {
            return Err(format!("pattern scan found {} matches (expected 1), the game build maybe unsupported", results.len()).into());
        }

        let target_address = *results.first()
            .ok_or_else(|| Box::<dyn Error>::from("pattern scan found no matches; the game build may be unsupported"))? as *mut c_void;
        let dll_config = DllConfig { target_address, enable_roll: config.enable_roll };
        inject_dll(*process_handle, config.dll_path, dll_config)?;

        ResumeThread(*thread_handle);
    }

    Ok(())
}

fn encode_wide(os_str: impl AsRef<OsStr>) -> Vec<u16> {
    os_str.as_ref().encode_wide().chain(iter::once(0)).collect()
}

fn report_error(msg: &str) {
    if !write_to_stderr(msg) {
        message_box("Error", msg);
    }
}

fn write_to_stderr(msg: &str) -> bool {
    fn is_valid_handle(handle: *mut c_void) -> bool {
        !handle.is_null() && handle != INVALID_HANDLE_VALUE.0
    }

    if is_valid_handle(io::stderr().as_raw_handle()) {
        return writeln!(io::stderr(), "error: {msg}").is_ok();
    }

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() && is_valid_handle(io::stderr().as_raw_handle()) {
            return writeln!(io::stderr(), "error: {msg}").is_ok()
        }
    }

    false
}

fn message_box(title: &str, message: &str) {
    let title = encode_wide(title);
    let message = encode_wide(message);

    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND
        );
    }
}

fn create_process(exe_path: impl AsRef<Path>, create_suspended: bool) -> WinResult<(Owned<HANDLE>, Owned<HANDLE>)> {
    let path_buf: Vec<u16> = encode_wide(exe_path.as_ref().as_os_str());
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
    let path_buf: Vec<u16> = encode_wide(dll_path.as_os_str());
    let dll_file_name = dll_path.file_name().unwrap_or_default();
    let name_buf: Vec<u16> = encode_wide(dll_file_name);
    unsafe {
        let kernel32_handle = GetModuleHandleW(w!("kernel32.dll"))?;
        let load_library = GetProcAddress(kernel32_handle, s!("LoadLibraryW")).ok_or_else(|| windows::core::Error::from_thread())?;

        call_remote(process_handle, Some(mem::transmute(load_library)), path_buf.as_slice())?;
        
        let dll_handle = Owned::new(LoadLibraryW(PCWSTR(path_buf.as_ptr()))?);
        let Some(initialize) = GetProcAddress(*dll_handle, s!("initialize")) else {
            return Err(windows::core::Error::from_thread());
        };

        let offset = initialize as usize - dll_handle.0 as usize;

        let remote_module = get_remote_module(process_handle, PCWSTR(name_buf.as_ptr()))?;
        let remote_initialize = remote_module.0.byte_add(offset);

        call_remote(process_handle, Some(mem::transmute(remote_initialize)), &config)?;
    }

    Ok(())
}

unsafe fn call_remote_raw(process_handle: HANDLE, remote_proc_address: LPTHREAD_START_ROUTINE, parameter: Option<*const c_void>) -> WinResult<u32> {
    unsafe {
        let thread_handle = Owned::new(CreateRemoteThread(process_handle, None, 0, remote_proc_address, parameter, 0, None)?);
        if WaitForSingleObject(*thread_handle, INFINITE) == WAIT_FAILED {
            return Err(windows::core::Error::from_thread());
        }
    
        let mut result = 0;
        GetExitCodeThread(*thread_handle, &mut result)?;

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