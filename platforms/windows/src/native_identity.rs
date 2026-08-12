//! Narrow Windows FFI boundary for current-process identity evidence.

use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetLengthSid, GetTokenInformation, IsValidSid, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER,
    TokenElevation, TokenUser,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcessToken,
};

const MAX_TOKEN_BYTES: u32 = 16 * 1024;
const MAX_EXECUTABLE_PATH_UNITS: usize = 32_768;
const MAX_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;

/// Stable content-free native process-identity failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsNativeIdentityError {
    /// The current process token could not be opened.
    TokenUnavailable,
    /// The token user identity was malformed or changed.
    UserIdentityUnavailable,
    /// The token elevation state could not be observed.
    ElevationUnavailable,
    /// The current Windows session could not be observed.
    SessionUnavailable,
    /// The current executable could not be identified and hashed within bounds.
    ExecutableUnavailable,
}

impl WindowsNativeIdentityError {
    /// Returns the stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::TokenUnavailable => "windows.identity.token_unavailable",
            Self::UserIdentityUnavailable => "windows.identity.user_unavailable",
            Self::ElevationUnavailable => "windows.identity.elevation_unavailable",
            Self::SessionUnavailable => "windows.identity.session_unavailable",
            Self::ExecutableUnavailable => "windows.identity.executable_unavailable",
        }
    }
}

impl fmt::Display for WindowsNativeIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WindowsNativeIdentityError {}

/// Redacted exact identity of the current native Windows process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsProcessIdentity {
    process_id: u32,
    session_id: u32,
    elevated: bool,
    user_sid_sha256: [u8; 32],
    executable_sha256: [u8; 32],
}

impl WindowsProcessIdentity {
    /// Returns the kernel process identifier.
    #[must_use]
    pub const fn process_id(&self) -> u32 {
        self.process_id
    }

    /// Returns the Windows session identifier.
    #[must_use]
    pub const fn session_id(&self) -> u32 {
        self.session_id
    }

    /// Returns whether the current token is elevated.
    #[must_use]
    pub const fn elevated(&self) -> bool {
        self.elevated
    }

    /// Returns only the SHA-256 digest of the user SID.
    #[must_use]
    pub const fn user_sid_sha256(&self) -> &[u8; 32] {
        &self.user_sid_sha256
    }

    /// Returns only the SHA-256 digest of the current executable bytes.
    #[must_use]
    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }

    /// Matches an expected process without exposing a user SID or executable path.
    #[must_use]
    pub fn matches(&self, expected: &Self) -> bool {
        self.process_id == expected.process_id
            && self.session_id == expected.session_id
            && self.elevated == expected.elevated
            && self.user_sid_sha256 == expected.user_sid_sha256
            && self.executable_sha256 == expected.executable_sha256
    }
}

/// Observes the current native Windows process through kernel-backed APIs.
pub fn observe_windows_process_identity()
-> Result<WindowsProcessIdentity, WindowsNativeIdentityError> {
    // SAFETY: `GetCurrentProcess` takes no pointer and returns a process pseudo-handle.
    let process = unsafe { GetCurrentProcess() };
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: `token` is a valid writable handle slot and the pseudo-handle is valid here.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 || token.is_null() {
        return Err(WindowsNativeIdentityError::TokenUnavailable);
    }
    let token = OwnedHandle(token);
    let user_sid_sha256 = token_user_digest(token.0)?;
    let elevated = token_elevation(token.0)?;
    // SAFETY: `GetCurrentProcessId` has no pointer preconditions.
    let process_id = unsafe { GetCurrentProcessId() };
    let mut session_id = 0_u32;
    // SAFETY: `session_id` is a valid writable `u32` for the current live PID.
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return Err(WindowsNativeIdentityError::SessionUnavailable);
    }
    let executable_sha256 = current_executable_digest()?;
    Ok(WindowsProcessIdentity {
        process_id,
        session_id,
        elevated,
        user_sid_sha256,
        executable_sha256,
    })
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this wrapper owns one real token handle exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn token_user_digest(token: HANDLE) -> Result<[u8; 32], WindowsNativeIdentityError> {
    let mut required = 0_u32;
    // SAFETY: a null output with zero length is the documented size query shape.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut required) };
    if required == 0 || required > MAX_TOKEN_BYTES {
        return Err(WindowsNativeIdentityError::UserIdentityUnavailable);
    }
    let mut buffer = vec![0_u8; required as usize];
    let mut observed = 0_u32;
    // SAFETY: the buffer has exactly `required` writable bytes and `observed` is writable.
    if unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut observed,
        )
    } == 0
        || observed != required
        || buffer.len() < size_of::<TOKEN_USER>()
    {
        return Err(WindowsNativeIdentityError::UserIdentityUnavailable);
    }
    // SAFETY: successful `TokenUser` output begins with an initialized `TOKEN_USER`.
    let sid = unsafe { (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    // SAFETY: the SID pointer originates from the successful token-information buffer.
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(WindowsNativeIdentityError::UserIdentityUnavailable);
    }
    // SAFETY: `sid` was validated by `IsValidSid`.
    let sid_length = unsafe { GetLengthSid(sid) } as usize;
    let start = sid as usize;
    let buffer_start = buffer.as_ptr() as usize;
    let buffer_end = buffer_start
        .checked_add(buffer.len())
        .ok_or(WindowsNativeIdentityError::UserIdentityUnavailable)?;
    let sid_end = start
        .checked_add(sid_length)
        .ok_or(WindowsNativeIdentityError::UserIdentityUnavailable)?;
    if sid_length == 0 || start < buffer_start || sid_end > buffer_end {
        return Err(WindowsNativeIdentityError::UserIdentityUnavailable);
    }
    // SAFETY: the validated SID range lies wholly inside the retained token buffer.
    let sid_bytes = unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), sid_length) };
    Ok(Sha256::digest(sid_bytes).into())
}

fn token_elevation(token: HANDLE) -> Result<bool, WindowsNativeIdentityError> {
    let mut elevation = TOKEN_ELEVATION::default();
    let mut observed = 0_u32;
    let expected = u32::try_from(size_of::<TOKEN_ELEVATION>())
        .map_err(|_| WindowsNativeIdentityError::ElevationUnavailable)?;
    // SAFETY: `elevation` and `observed` are valid writable outputs of exact size.
    if unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            expected,
            &mut observed,
        )
    } == 0
        || observed != expected
    {
        return Err(WindowsNativeIdentityError::ElevationUnavailable);
    }
    Ok(elevation.TokenIsElevated != 0)
}

fn current_executable_digest() -> Result<[u8; 32], WindowsNativeIdentityError> {
    let mut path = vec![0_u16; MAX_EXECUTABLE_PATH_UNITS];
    // SAFETY: `path` is a valid writable UTF-16 buffer and null module means current image.
    let length = unsafe {
        GetModuleFileNameW(
            std::ptr::null_mut(),
            path.as_mut_ptr(),
            u32::try_from(path.len())
                .map_err(|_| WindowsNativeIdentityError::ExecutableUnavailable)?,
        )
    } as usize;
    if length == 0 || length >= path.len() {
        return Err(WindowsNativeIdentityError::ExecutableUnavailable);
    }
    path.truncate(length);
    let path = PathBuf::from(OsString::from_wide(&path));
    let mut file =
        File::open(path).map_err(|_| WindowsNativeIdentityError::ExecutableUnavailable)?;
    let metadata = file
        .metadata()
        .map_err(|_| WindowsNativeIdentityError::ExecutableUnavailable)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_EXECUTABLE_BYTES {
        return Err(WindowsNativeIdentityError::ExecutableUnavailable);
    }
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut block = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut block)
            .map_err(|_| WindowsNativeIdentityError::ExecutableUnavailable)?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(count as u64)
            .ok_or(WindowsNativeIdentityError::ExecutableUnavailable)?;
        if observed > MAX_EXECUTABLE_BYTES {
            return Err(WindowsNativeIdentityError::ExecutableUnavailable);
        }
        hasher.update(&block[..count]);
    }
    if observed != metadata.len() {
        return Err(WindowsNativeIdentityError::ExecutableUnavailable);
    }
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::observe_windows_process_identity;

    #[test]
    fn native_process_identity_is_stable_redacted_and_exact() {
        let first = observe_windows_process_identity().expect("native Windows identity");
        let second = observe_windows_process_identity().expect("repeated native Windows identity");
        assert!(first.process_id() > 0);
        assert!(first.matches(&second));
        assert_ne!(first.user_sid_sha256(), &[0; 32]);
        assert_ne!(first.executable_sha256(), &[0; 32]);
        let debug = format!("{first:?}");
        assert!(!debug.contains("S-1-"));
        assert!(!debug.to_ascii_lowercase().contains(".exe"));
    }
}
