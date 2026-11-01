use std::{ffi::c_void, fmt::Display};

use minhook_sys::{
    MH_STATUS, MH_STATUS_MH_ERROR_ALREADY_CREATED, MH_STATUS_MH_ERROR_ALREADY_INITIALIZED,
    MH_STATUS_MH_ERROR_DISABLED, MH_STATUS_MH_ERROR_ENABLED, MH_STATUS_MH_ERROR_FUNCTION_NOT_FOUND,
    MH_STATUS_MH_ERROR_MEMORY_ALLOC, MH_STATUS_MH_ERROR_MEMORY_PROTECT, MH_STATUS_MH_ERROR_MODULE_NOT_FOUND,
    MH_STATUS_MH_ERROR_NOT_CREATED, MH_STATUS_MH_ERROR_NOT_EXECUTABLE, MH_STATUS_MH_ERROR_NOT_INITIALIZED,
    MH_STATUS_MH_ERROR_UNSUPPORTED_FUNCTION, MH_STATUS_MH_OK
};

#[repr(C)]
pub struct InitParams {
    pub target_address: *mut c_void,
    pub enable_roll: bool
}

#[repr(u32)]
pub enum InitStatus {
    Ok                               = 0,
    AlreadyStarted                   = 1,
    FreeTrackInitError               = 2,
    ParameterIsNull                  = 3,
    MhStatusUnknown                  = 4,
    MhStatusErrorAlreadyInitialized  = 5,
    MhStatusErrorNotInitialized      = 6,
    MhStatusErrorAlreadyCreated      = 7,
    MhStatusErrorNotCreated          = 8,
    MhStatusErrorEnabled             = 9,
    MhStatusErrorDisabled            = 10,
    MhStatusErrorNotExecutable       = 11,
    MhStatusErrorUnsupportedFunction = 12,
    MhStatusErrorMemoryAlloc         = 13,
    MhStatusErrorMemoryProtect       = 14,
    MhStatusErrorModuleNotFound      = 15,
    MhStatusErrorFunctionNotFound    = 16
}

impl InitStatus {
    #[allow(dead_code)]
    pub fn is_ok(&self) -> bool {
        if let Self::Ok = self {
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn from_mh_status(status: MH_STATUS) -> Self {
        match status {
            MH_STATUS_MH_OK => Self::Ok,
            MH_STATUS_MH_ERROR_ALREADY_INITIALIZED => Self::MhStatusErrorAlreadyInitialized,
            MH_STATUS_MH_ERROR_NOT_INITIALIZED => Self::MhStatusErrorNotInitialized,
            MH_STATUS_MH_ERROR_ALREADY_CREATED => Self::MhStatusErrorAlreadyCreated,
            MH_STATUS_MH_ERROR_NOT_CREATED => Self::MhStatusErrorNotCreated,
            MH_STATUS_MH_ERROR_ENABLED => Self::MhStatusErrorEnabled,
            MH_STATUS_MH_ERROR_DISABLED => Self::MhStatusErrorDisabled,
            MH_STATUS_MH_ERROR_NOT_EXECUTABLE => Self::MhStatusErrorNotExecutable,
            MH_STATUS_MH_ERROR_UNSUPPORTED_FUNCTION => Self::MhStatusErrorUnsupportedFunction,
            MH_STATUS_MH_ERROR_MEMORY_ALLOC => Self::MhStatusErrorMemoryAlloc,
            MH_STATUS_MH_ERROR_MEMORY_PROTECT => Self::MhStatusErrorMemoryProtect,
            MH_STATUS_MH_ERROR_MODULE_NOT_FOUND => Self::MhStatusErrorModuleNotFound,
            MH_STATUS_MH_ERROR_FUNCTION_NOT_FOUND => Self::MhStatusErrorFunctionNotFound,
            _ => Self::MhStatusUnknown
        }
    }
}

impl TryFrom<u32> for InitStatus {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ok),
            1 => Ok(Self::AlreadyStarted),
            2 => Ok(Self::FreeTrackInitError),
            3 => Ok(Self::ParameterIsNull),
            4 => Ok(Self::MhStatusUnknown),
            5 => Ok(Self::MhStatusErrorAlreadyInitialized),
            6 => Ok(Self::MhStatusErrorNotInitialized),
            7 => Ok(Self::MhStatusErrorAlreadyCreated),
            8 => Ok(Self::MhStatusErrorNotCreated),
            9 => Ok(Self::MhStatusErrorEnabled),
            10 => Ok(Self::MhStatusErrorDisabled),
            11 => Ok(Self::MhStatusErrorNotExecutable),
            12 => Ok(Self::MhStatusErrorUnsupportedFunction),
            13 => Ok(Self::MhStatusErrorMemoryAlloc),
            14 => Ok(Self::MhStatusErrorMemoryProtect),
            15 => Ok(Self::MhStatusErrorModuleNotFound),
            16 => Ok(Self::MhStatusErrorFunctionNotFound),
            _ => Err("unknown status")
        }
    }
}

impl Display for InitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitStatus::Ok => write!(f, "success"),
            InitStatus::AlreadyStarted => write!(f, "initialization was already started by an earlier call"),
            InitStatus::FreeTrackInitError => write!(f, "failed to initialize the FreeTrack interface"),
            InitStatus::ParameterIsNull => write!(f, "initialization parameter is null"),
            InitStatus::MhStatusUnknown => write!(f, "minhook: unknown error"),
            InitStatus::MhStatusErrorAlreadyInitialized => write!(f, "minhook: already initialized"),
            InitStatus::MhStatusErrorNotInitialized => write!(f, "minhook: not initialized, or already uninitialized"),
            InitStatus::MhStatusErrorAlreadyCreated => write!(f, "minhook: hook for the target function already exists"),
            InitStatus::MhStatusErrorNotCreated => write!(f, "minhook: hook for the target function does not exist"),
            InitStatus::MhStatusErrorEnabled => write!(f, "minhook: hook is already enabled"),
            InitStatus::MhStatusErrorDisabled => write!(f, "minhook: hook is not enabled"),
            InitStatus::MhStatusErrorNotExecutable => write!(f, "minhook: target address is not allocated or not executable"),
            InitStatus::MhStatusErrorUnsupportedFunction => write!(f, "minhook: target function cannot be hooked"),
            InitStatus::MhStatusErrorMemoryAlloc => write!(f, "minhook: failed to allocate memory"),
            InitStatus::MhStatusErrorMemoryProtect => write!(f, "minhook: failed to change memory protection"),
            InitStatus::MhStatusErrorModuleNotFound => write!(f, "minhook: target module is not loaded"),
            InitStatus::MhStatusErrorFunctionNotFound => write!(f, "Tminhook: target function not found")
        }
    }
}