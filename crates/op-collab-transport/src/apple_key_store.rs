//! iOS Keychain entry for the collaboration device identity.
//!
//! Creation explicitly selects a `ThisDeviceOnly` accessibility class and
//! disables synchronization. The Noise identity remains available after the
//! first unlock without entering iCloud Keychain or a device backup.

#![cfg(all(feature = "native", target_os = "ios"))]

use std::ptr;
use std::sync::Arc;

use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFGetTypeID, CFRelease, CFTypeRef, OSStatus};
use core_foundation_sys::string::CFStringRef;
use security_framework_sys::access_control::kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly;
use security_framework_sys::base::{
    errSecAuthFailed, errSecDuplicateItem, errSecItemNotFound, errSecSuccess,
};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecAttrSynchronizable, kSecClass, kSecClassGenericPassword,
    kSecReturnData, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching};
use zeroize::{Zeroize, Zeroizing};

use crate::os_key_store::CredentialEntry;
use crate::KeyStoreError;

const KEYRING_SERVICE: &str = "tech.zseven.openpencil.collaboration";
const KEYRING_ACCOUNT: &str = "device-noise-x25519-v1";

const ERR_SEC_NOT_AVAILABLE: OSStatus = -25291;
const ERR_SEC_READ_ONLY: OSStatus = -25292;
const ERR_SEC_AUTH_FAILED: OSStatus = errSecAuthFailed;
const ERR_SEC_SUCCESS: OSStatus = errSecSuccess;
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecAttrAccessible: CFStringRef;
}

pub(crate) struct AppleKeychainEntry;

impl AppleKeychainEntry {
    pub(crate) const fn new() -> Result<Self, KeyStoreError> {
        Ok(Self)
    }
}

impl CredentialEntry for AppleKeychainEntry {
    fn load(&self) -> Result<Option<Zeroizing<String>>, KeyStoreError> {
        let mut query = key_query();
        query.push((
            unsafe { CFString::wrap_under_get_rule(kSecReturnData) },
            CFBoolean::true_value().into_CFType(),
        ));
        let query = CFDictionary::from_CFType_pairs(&query);
        let mut result: CFTypeRef = ptr::null();
        let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
        if status != errSecSuccess {
            if !result.is_null() {
                unsafe { CFRelease(result) };
            }
            if status == errSecItemNotFound {
                return Ok(None);
            }
            map_status(status)?;
            return Err(KeyStoreError::PlatformStoreFailure);
        }
        decode_result(result).map(Some)
    }

    fn store(&self, value: &str) -> Result<(), KeyStoreError> {
        let mut query = key_query();
        // Avoid an immutable CoreFoundation-owned secret copy: this CFData
        // borrows a Zeroizing buffer that is wiped when the query releases it.
        let value_data = CFData::from_arc(Arc::new(Zeroizing::new(value.as_bytes().to_vec())));
        query.extend([
            (
                unsafe { CFString::wrap_under_get_rule(kSecAttrAccessible) },
                unsafe {
                    CFString::wrap_under_get_rule(kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly)
                        .into_CFType()
                },
            ),
            (
                unsafe { CFString::wrap_under_get_rule(kSecValueData) },
                value_data.into_CFType(),
            ),
        ]);
        let query = CFDictionary::from_CFType_pairs(&query);
        let status = unsafe { SecItemAdd(query.as_concrete_TypeRef(), ptr::null_mut()) };

        // Another process won first creation. The caller immediately reloads
        // and validates the canonical entry, so this is success.
        if status == errSecDuplicateItem {
            return Ok(());
        }
        map_status(status)
    }
}

fn key_query() -> Vec<(CFString, CFType)> {
    vec![
        (
            unsafe { CFString::wrap_under_get_rule(kSecClass) },
            unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType() },
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrService) },
            CFString::from(KEYRING_SERVICE).into_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) },
            CFString::from(KEYRING_ACCOUNT).into_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrSynchronizable) },
            CFBoolean::false_value().into_CFType(),
        ),
    ]
}

fn decode_result(result: CFTypeRef) -> Result<Zeroizing<String>, KeyStoreError> {
    if result.is_null() {
        return Err(KeyStoreError::UnsafePlatformEntry);
    }
    if unsafe { CFGetTypeID(result) } != CFData::type_id() {
        unsafe { CFRelease(result) };
        return Err(KeyStoreError::UnsafePlatformEntry);
    }

    let data = unsafe { CFData::wrap_under_create_rule(result.cast()) };
    let mut bytes = Zeroizing::new(data.bytes().to_vec());
    match String::from_utf8(std::mem::take(&mut *bytes)) {
        Ok(value) => Ok(Zeroizing::new(value)),
        Err(error) => {
            let mut bytes = error.into_bytes();
            bytes.zeroize();
            Err(KeyStoreError::UnsafePlatformEntry)
        }
    }
}

fn map_status(status: OSStatus) -> Result<(), KeyStoreError> {
    match status {
        ERR_SEC_SUCCESS => Ok(()),
        ERR_SEC_NOT_AVAILABLE
        | ERR_SEC_READ_ONLY
        | ERR_SEC_AUTH_FAILED
        | ERR_SEC_INTERACTION_NOT_ALLOWED => Err(KeyStoreError::PlatformStoreAccessDenied),
        _ => Err(KeyStoreError::PlatformStoreFailure),
    }
}
