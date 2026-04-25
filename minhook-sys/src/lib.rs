#![allow(non_camel_case_types)]

use std::{ffi::{c_char, c_int, c_ushort, c_void}, ptr::null_mut};

/// MinHook Error Codes.
pub type MH_STATUS = c_int;

/// Unknown error. Should not be returned.
pub const MH_STATUS_MH_UNKNOWN: MH_STATUS = -1;

/// Successful.
pub const MH_STATUS_MH_OK: MH_STATUS = 0;

/// MinHook is already initialized.
pub const MH_STATUS_MH_ERROR_ALREADY_INITIALIZED: MH_STATUS = 1;

/// MinHook is not initialized yet, or already uninitialized.
pub const MH_STATUS_MH_ERROR_NOT_INITIALIZED: MH_STATUS = 2;

/// The hook for the specified target function is already created.
pub const MH_STATUS_MH_ERROR_ALREADY_CREATED: MH_STATUS = 3;

/// The hook for the specified target function is not created yet.
pub const MH_STATUS_MH_ERROR_NOT_CREATED: MH_STATUS = 4;

/// The hook for the specified target function is already enabled.
pub const MH_STATUS_MH_ERROR_ENABLED: MH_STATUS = 5;

/// The hook for the specified target function is not enabled yet, or already disabled.
pub const MH_STATUS_MH_ERROR_DISABLED: MH_STATUS = 6;

/// The specified pointer is invalid. It points the address of non-allocated
/// and/or non-executable region.
pub const MH_STATUS_MH_ERROR_NOT_EXECUTABLE: MH_STATUS = 7;

/// The specified target function cannot be hooked.
pub const MH_STATUS_MH_ERROR_UNSUPPORTED_FUNCTION: MH_STATUS = 8;

/// Failed to allocate memory.
pub const MH_STATUS_MH_ERROR_MEMORY_ALLOC: MH_STATUS = 9;

/// Failed to change the memory protection.
pub const MH_STATUS_MH_ERROR_MEMORY_PROTECT: MH_STATUS = 10;

/// The specified module is not loaded.
pub const MH_STATUS_MH_ERROR_MODULE_NOT_FOUND: MH_STATUS = 11;

/// The specified function is not found.
pub const MH_STATUS_MH_ERROR_FUNCTION_NOT_FOUND: MH_STATUS = 12;

/// Can be passed as a parameter to MH_EnableHook, MH_DisableHook,
/// MH_QueueEnableHook or MH_QueueDisableHook.
pub const MH_ALL_HOOKS: *mut c_void = null_mut();

unsafe extern "C" {
    /// Initialize the MinHook library. You must call this function EXACTLY ONCE
    /// at the beginning of your program.
    pub fn MH_Initialize() -> MH_STATUS;

    /// Uninitialize the MinHook library. You must call this function EXACTLY
    /// ONCE at the end of your program.    
    pub fn MH_Uninitialize() -> MH_STATUS;

    /// Creates a hook for the specified target function, in disabled state.
    /// 
    /// # Parameters:
    /// * pTarget     [in]  A pointer to the target function, which will be
    ///                     overridden by the detour function.
    /// * pDetour     [in]  A pointer to the detour function, which will override
    ///                     the target function.
    /// * ppOriginal  [out] A pointer to the trampoline function, which will be
    ///                     used to call the original target function.
    ///                     This parameter can be NULL.
    pub fn MH_CreateHook(pTarget: *mut c_void, pDetour: *mut c_void, ppOriginal: *mut *mut c_void) -> MH_STATUS;

    /// Creates a hook for the specified API function, in disabled state.
    ///
    /// # Parameters:
    /// * pszModule   [in]  A pointer to the loaded module name which contains the
    ///                     target function.
    /// * pszProcName [in]  A pointer to the target function name, which will be
    ///                     overridden by the detour function.
    /// * pDetour     [in]  A pointer to the detour function, which will override
    ///                     the target function.
    /// * ppOriginal  [out] A pointer to the trampoline function, which will be
    ///                     used to call the original target function.
    ///                     This parameter can be NULL.
    pub fn MH_CreateHookApi(
        pszModule: *const c_ushort,
        pszProcName: *const c_char,
        pDetour: *mut c_void,
        ppOriginal: *mut *mut c_void
    ) -> MH_STATUS;

    /// Creates a hook for the specified API function, in disabled state.
    ///
    /// # Parameters:
    /// * pszModule   [in]  A pointer to the loaded module name which contains the
    ///                     target function.
    /// * pszProcName [in]  A pointer to the target function name, which will be
    ///                     overridden by the detour function.
    /// * pDetour     [in]  A pointer to the detour function, which will override
    ///                     the target function.
    /// * ppOriginal  [out] A pointer to the trampoline function, which will be
    ///                     used to call the original target function.
    ///                     This parameter can be NULL.
    /// * ppTarget    [out] A pointer to the target function, which will be used
    ///                     with other functions.
    ///                     This parameter can be NULL.
    pub fn MH_CreateHookApiEx(
        pszModule: *const c_ushort,
        pszProcName: *const c_char,
        pDetour: *mut c_void,
        ppOriginal: *mut *mut c_void,
        ppTarget: *mut *mut c_void
    ) -> MH_STATUS;

    /// Removes an already created hook.
    ///
    /// # Parameters:
    /// * pTarget [in] A pointer to the target function.
    pub fn MH_RemoveHook(pTarget: *mut c_void) -> MH_STATUS;

    /// Enables an already created hook.
    ///
    /// # Parameters:
    /// * pTarget [in] A pointer to the target function.
    ///                If this parameter is MH_ALL_HOOKS, all created hooks are
    ///                enabled in one go.
    pub fn MH_EnableHook(pTarget: *mut c_void) -> MH_STATUS;

    /// Disables an already created hook.
    ///
    /// # Parameters:
    /// * pTarget [in] A pointer to the target function.
    ///                If this parameter is MH_ALL_HOOKS, all created hooks are
    ///                disabled in one go.
    pub fn MH_DisableHook(pTarget: *mut c_void) -> MH_STATUS;

    /// Queues to enable an already created hook.
    ///
    /// # Parameters:
    /// * pTarget [in] A pointer to the target function.
    ///                If this parameter is MH_ALL_HOOKS, all created hooks are
    ///                queued to be enabled.
    pub fn MH_QueueEnableHook(pTarget: *mut c_void) -> MH_STATUS;

    /// Queues to disable an already created hook.
    ///
    /// # Parameters:
    /// * pTarget [in] A pointer to the target function.
    ///                If this parameter is MH_ALL_HOOKS, all created hooks are
    ///                queued to be disabled.
    pub fn MH_QueueDisableHook(pTarget: *mut c_void) -> MH_STATUS;

    /// Applies all queued changes in one go.
    pub fn MH_ApplyQueued() -> MH_STATUS;

    /// Translates the MH_STATUS to its name as a string.
    pub fn MH_StatusToString(status: MH_STATUS) -> *const c_char;
}