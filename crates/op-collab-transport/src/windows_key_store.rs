use std::ptr::NonNull;

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ACCESS_DENIED, ERROR_NOT_FOUND, ERROR_NO_SUCH_LOGON_SESSION,
};
use windows_sys::Win32::Security::Credentials::{
    CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_MAX_CREDENTIAL_BLOB_SIZE,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};
use zeroize::{Zeroize, Zeroizing};

use crate::os_key_store::CredentialEntry;
use crate::KeyStoreError;

const TARGET_NAME: &str = "OpenPencil.Collaboration.device-noise-x25519-v1";
const USER_NAME: &str = "device-noise-x25519-v1";

pub(crate) struct WindowsCredentialEntry;

impl WindowsCredentialEntry {
    pub(crate) fn new() -> Result<Self, KeyStoreError> {
        Ok(Self)
    }
}

impl CredentialEntry for WindowsCredentialEntry {
    fn load(&self) -> Result<Option<Zeroizing<String>>, KeyStoreError> {
        let target_name = wide_string(TARGET_NAME);
        let mut raw = std::ptr::null_mut();
        let read = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
        if read == 0 {
            return map_read_error(unsafe { GetLastError() });
        }

        let guard = CredentialGuard(NonNull::new(raw).ok_or(KeyStoreError::PlatformStoreFailure)?);
        let credential = unsafe { guard.0.as_ref() };
        let length = credential.CredentialBlobSize as usize;
        if length > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
            || (length != 0 && credential.CredentialBlob.is_null())
        {
            return Err(KeyStoreError::UnsafePlatformEntry);
        }

        let bytes = if length == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(credential.CredentialBlob, length) }
        };
        std::str::from_utf8(bytes).map_err(|_| KeyStoreError::UnsafePlatformEntry)?;
        let mut owned = Zeroizing::new(bytes.to_vec());
        let value = unsafe { String::from_utf8_unchecked(std::mem::take(&mut *owned)) };
        Ok(Some(Zeroizing::new(value)))
    }

    fn store(&self, value: &str) -> Result<(), KeyStoreError> {
        let mut target_name = wide_string(TARGET_NAME);
        let mut user_name = wide_string(USER_NAME);
        let mut secret = Zeroizing::new(value.as_bytes().to_vec());
        if secret.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
            return Err(KeyStoreError::PlatformStoreFailure);
        }

        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target_name.as_mut_ptr(),
            CredentialBlobSize: secret.len() as u32,
            CredentialBlob: secret.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: user_name.as_mut_ptr(),
            ..CREDENTIALW::default()
        };
        let written = unsafe { CredWriteW(&credential, 0) };
        secret.zeroize();
        if written == 0 {
            return Err(map_write_error(unsafe { GetLastError() }));
        }
        Ok(())
    }
}

struct CredentialGuard(NonNull<CREDENTIALW>);

impl Drop for CredentialGuard {
    fn drop(&mut self) {
        let credential = unsafe { self.0.as_ref() };
        let length = credential.CredentialBlobSize as usize;
        if length <= CRED_MAX_CREDENTIAL_BLOB_SIZE as usize && !credential.CredentialBlob.is_null()
        {
            for index in 0..length {
                unsafe {
                    std::ptr::write_volatile(credential.CredentialBlob.add(index), 0);
                }
            }
        }
        unsafe {
            CredFree(self.0.as_ptr().cast());
        }
    }
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn map_read_error(code: u32) -> Result<Option<Zeroizing<String>>, KeyStoreError> {
    match code {
        ERROR_NOT_FOUND => Ok(None),
        ERROR_ACCESS_DENIED | ERROR_NO_SUCH_LOGON_SESSION => {
            Err(KeyStoreError::PlatformStoreAccessDenied)
        }
        _ => Err(KeyStoreError::PlatformStoreFailure),
    }
}

fn map_write_error(code: u32) -> KeyStoreError {
    match code {
        ERROR_ACCESS_DENIED | ERROR_NO_SUCH_LOGON_SESSION => {
            KeyStoreError::PlatformStoreAccessDenied
        }
        _ => KeyStoreError::PlatformStoreFailure,
    }
}
