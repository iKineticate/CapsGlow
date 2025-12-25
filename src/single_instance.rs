use crate::{config::EXE_NAME, util::to_wide};

use anyhow::{Context, Result, anyhow};
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::{CreateMutexW, ReleaseMutex},
    },
    core::PCWSTR,
};

pub struct SingleInstance {
    handle: Option<HANDLE>,
}

impl SingleInstance {
    /// Creates a new system-wide mutex to ensure that only one instance of
    /// the application is running.
    pub fn new() -> Result<Self> {
        let name = to_wide(EXE_NAME.as_str());

        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
            .context("Failed to create single instance mutex.")?;

        if handle.is_invalid() {
            return Err(anyhow!(
                "Failed to create single instance mutex: {:?}",
                unsafe { GetLastError() }
            ));
        }

        let handle = if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Err(anyhow!(
                "CapsGlow already running, exit the new process: {:?}",
                unsafe { GetLastError() }
            ));
        } else {
            Some(handle)
        };

        Ok(SingleInstance { handle })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            if let Some(handle) = self.handle {
                let _ = ReleaseMutex(handle);
                let _ = CloseHandle(handle);
            }
        }
    }
}
