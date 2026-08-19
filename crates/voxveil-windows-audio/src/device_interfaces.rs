#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TopologyCandidate {
    pub(crate) device_instance_id: String,
    pub(crate) interface_path: String,
    pub(crate) alias_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CandidateSelection {
    Unique(TopologyCandidate),
    Ambiguous,
    None,
}

pub(crate) fn select_topology_candidate(
    adapter_device_id: &str,
    candidates: &[TopologyCandidate],
) -> CandidateSelection {
    let same_device: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .device_instance_id
                .eq_ignore_ascii_case(adapter_device_id)
                || candidate.interface_path.eq_ignore_ascii_case(adapter_device_id)
        })
        .cloned()
        .collect();

    if same_device.is_empty() {
        return CandidateSelection::None;
    }

    let alias_matches: Vec<_> = same_device
        .iter()
        .filter(|candidate| candidate.alias_match)
        .cloned()
        .collect();

    match alias_matches.len() {
        1 => CandidateSelection::Unique(alias_matches[0].clone()),
        n if n > 1 => CandidateSelection::Ambiguous,
        _ => match same_device.len() {
            1 => CandidateSelection::Unique(same_device[0].clone()),
            _ => CandidateSelection::Ambiguous,
        },
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_runtime {
    use std::mem::size_of;

    use windows::{
        Win32::Devices::DeviceAndDriverInstallation::{
            DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO, SP_DEVICE_INTERFACE_DATA,
            SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
            SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
            SetupDiGetDeviceInterfaceAlias, SetupDiGetDeviceInterfaceDetailW,
        },
        core::{GUID, PCWSTR},
    };

    use super::TopologyCandidate;

    const KSCATEGORY_TOPOLOGY: GUID =
        GUID::from_u128(0xdda54a40_1e4c_11d1_a050_405705c10000);
    const KSCATEGORY_AUDIO: GUID = GUID::from_u128(0x6994ad04_93ef_11d0_a3cc_00a0c9223196);

    pub(crate) fn enumerate_topology_interfaces() -> Result<Vec<TopologyCandidate>, String> {
        unsafe {
            let set = SetupDiGetClassDevsW(
                Some(&KSCATEGORY_TOPOLOGY),
                PCWSTR::null(),
                None,
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
            .map(DeviceInfoSet)
            .map_err(|error| format!("failed to enumerate topology interface class: {error}"))?;

            let mut candidates = Vec::new();
            let mut index = 0u32;
            loop {
                let mut interface_data = SP_DEVICE_INTERFACE_DATA {
                    cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                    ..Default::default()
                };
                if SetupDiEnumDeviceInterfaces(
                    set.0,
                    None,
                    &KSCATEGORY_TOPOLOGY,
                    index,
                    &mut interface_data,
                )
                .is_err()
                {
                    break;
                }
                index += 1;

                let mut required_size = 0u32;
                let _ = SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface_data,
                    None,
                    0,
                    Some(&mut required_size),
                    None,
                );
                if required_size == 0 {
                    continue;
                }

                let mut detail_buffer = vec![0u8; required_size as usize];
                let detail = detail_buffer
                    .as_mut_ptr()
                    .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
                (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
                let mut device_info = SP_DEVINFO_DATA {
                    cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                    ..Default::default()
                };
                SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface_data,
                    Some(detail),
                    required_size,
                    None,
                    Some(&mut device_info),
                )
                .map_err(|error| format!("failed to read topology interface detail: {error}"))?;

                let interface_path = PCWSTR::from_raw((*detail).DevicePath.as_ptr())
                    .to_string()
                    .map_err(|error| format!("invalid topology interface path: {error}"))?;
                let device_instance_id = device_instance_id(set.0, &device_info)?;

                let mut alias_data = SP_DEVICE_INTERFACE_DATA {
                    cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                    ..Default::default()
                };
                let alias_match = SetupDiGetDeviceInterfaceAlias(
                    set.0,
                    &interface_data,
                    &KSCATEGORY_AUDIO,
                    &mut alias_data,
                )
                .is_ok();

                candidates.push(TopologyCandidate {
                    device_instance_id,
                    interface_path,
                    alias_match,
                });
            }

            Ok(candidates)
        }
    }

    unsafe fn device_instance_id(
        set: HDEVINFO,
        device_info: &SP_DEVINFO_DATA,
    ) -> Result<String, String> {
        let mut required = 0u32;
        let _ = unsafe {
            SetupDiGetDeviceInstanceIdW(set, device_info, None, Some(&mut required))
        };
        if required == 0 {
            return Err("topology interface device instance ID was unavailable".into());
        }
        let mut buffer = vec![0u16; required as usize];
        unsafe {
            SetupDiGetDeviceInstanceIdW(set, device_info, Some(buffer.as_mut_slice()), None)
        }
        .map_err(|error| format!("failed to read topology device instance ID: {error}"))?;
        let length = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
        Ok(String::from_utf16_lossy(&buffer[..length]))
    }

    struct DeviceInfoSet(HDEVINFO);

    impl Drop for DeviceInfoSet {
        fn drop(&mut self) {
            unsafe {
                let _ = SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }
}

#[cfg(windows)]
pub(crate) use windows_runtime::enumerate_topology_interfaces;

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(device_instance_id: &str, interface_path: &str, alias_match: bool) -> TopologyCandidate {
        TopologyCandidate {
            device_instance_id: device_instance_id.into(),
            interface_path: interface_path.into(),
            alias_match,
        }
    }

    #[test]
    fn unique_same_device_candidate_wins() {
        let expected = candidate("HDAUDIO\\FUNC_01", "topology-a", false);
        let selection = select_topology_candidate(
            "hdaudio\\func_01",
            &[
                expected.clone(),
                candidate("HDAUDIO\\OTHER", "topology-b", true),
            ],
        );
        assert_eq!(selection, CandidateSelection::Unique(expected));
    }

    #[test]
    fn adapter_interface_path_selects_owning_devnode() {
        let expected = candidate(
            "HDAUDIO\\FUNC_01",
            "\\\\?\\hdaudio#func_01#{dda54a40-1e4c-11d1-a050-405705c10000}",
            true,
        );
        let selection = select_topology_candidate(
            "\\\\?\\HDAUDIO#FUNC_01#{DDA54A40-1E4C-11D1-A050-405705C10000}",
            &[expected.clone()],
        );
        assert_eq!(selection, CandidateSelection::Unique(expected));
    }

    #[test]
    fn alias_match_wins_among_same_device_candidates() {
        let expected = candidate("HDAUDIO\\FUNC_01", "topology-alias", true);
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[
                candidate("HDAUDIO\\FUNC_01", "topology-plain", false),
                expected.clone(),
            ],
        );
        assert_eq!(selection, CandidateSelection::Unique(expected));
    }

    #[test]
    fn indistinguishable_same_device_candidates_are_ambiguous() {
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[
                candidate("HDAUDIO\\FUNC_01", "topology-a", false),
                candidate("hdaudio\\func_01", "topology-b", false),
            ],
        );
        assert_eq!(selection, CandidateSelection::Ambiguous);
    }

    #[test]
    fn unrelated_candidates_do_not_match() {
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[candidate("HDAUDIO\\OTHER", "topology-b", true)],
        );
        assert_eq!(selection, CandidateSelection::None);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_topology_interface_enumeration_is_structured() {
        assert!(enumerate_topology_interfaces().is_ok());
    }
}
