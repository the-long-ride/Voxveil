use std::ffi::c_void;

use windows::{
    Win32::{
        Media::Audio::{IDeviceTopology, IMMDeviceEnumerator, MMDeviceEnumerator},
        System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
            CoUninitialize,
        },
    },
    core::{HSTRING, PWSTR},
};

const RPC_E_CHANGED_MODE: i32 = 0x8001_0106u32 as i32;

pub(crate) fn normalize_device_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn resolve_adapter_device_id(endpoint_id: &str) -> Result<Option<String>, String> {
    if endpoint_id.trim().is_empty() {
        return Ok(None);
    }

    let _apartment = ComApartment::enter()?;
    let endpoint_id = HSTRING::from(endpoint_id);

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| format!("failed to create MMDeviceEnumerator: {error}"))?;
        let device = match enumerator.GetDevice(&endpoint_id) {
            Ok(device) => device,
            Err(_) => return Ok(None),
        };
        let topology: IDeviceTopology = match device.Activate(CLSCTX_ALL, None) {
            Ok(topology) => topology,
            Err(_) => return Ok(None),
        };
        let connector_count = match topology.GetConnectorCount() {
            Ok(count) => count,
            Err(_) => return Ok(None),
        };

        for index in 0..connector_count {
            let Ok(connector) = topology.GetConnector(index) else {
                continue;
            };
            let Ok(device_id) = connector.GetDeviceIdConnectedTo() else {
                continue;
            };
            let value = pwstr_to_string_and_free(device_id)?;
            let value = normalize_device_id(&value);
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
    }

    Ok(None)
}

struct ComApartment {
    owns_initialization: bool,
}

impl ComApartment {
    fn enter() -> Result<Self, String> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result.is_ok() {
            return Ok(Self {
                owns_initialization: true,
            });
        }
        if result.0 == RPC_E_CHANGED_MODE {
            return Ok(Self {
                owns_initialization: false,
            });
        }
        Err(format!("failed to initialize COM for audio topology: {result:?}"))
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.owns_initialization {
            unsafe { CoUninitialize() };
        }
    }
}

fn pwstr_to_string_and_free(value: PWSTR) -> Result<String, String> {
    if value.0.is_null() {
        return Ok(String::new());
    }
    let converted = unsafe { value.to_string() };
    unsafe { CoTaskMemFree(Some(value.0.cast::<c_void>())) };
    converted.map_err(|error| format!("invalid DeviceTopology device ID: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_device_identity_case_insensitively() {
        assert_eq!(normalize_device_id("HDAUDIO\\FUNC_01"), "hdaudio\\func_01");
    }

    #[test]
    fn empty_endpoint_id_has_no_adapter_binding() {
        assert_eq!(resolve_adapter_device_id(""), Ok(None));
    }
}
