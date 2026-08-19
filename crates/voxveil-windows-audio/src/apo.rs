use crate::{BackendProbe, RelayReadiness};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CONTROL_VERSION: u8 = 1;
const RUNTIME_VERSION: u8 = 1;
const RUNTIME_SIZE: usize = 21;
const HEARTBEAT_MAX_AGE_MS: u64 = 5_000;

#[derive(Clone, Debug)]
struct ApoPaths {
    marker: PathBuf,
    control: PathBuf,
    runtime: PathBuf,
    dll: PathBuf,
}

impl ApoPaths {
    fn system() -> Self {
        let program_data = env_path("ProgramData", r"C:\ProgramData");
        let program_files = env_path("ProgramFiles", r"C:\Program Files");
        Self::from_roots(program_data.join("Voxveil"), program_files.join("Voxveil"))
    }

    fn from_roots(state_root: PathBuf, install_root: PathBuf) -> Self {
        Self {
            marker: state_root.join("apo-installed.json"),
            control: state_root.join("apo-control.bin"),
            runtime: state_root.join("apo-runtime.bin"),
            dll: install_root.join("system-audio").join("VoxveilApo.dll"),
        }
    }
}

fn env_path(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(OsString::from(fallback)))
}

pub struct ApoBackend {
    paths: ApoPaths,
}

impl ApoBackend {
    pub fn new() -> Self {
        Self {
            paths: ApoPaths::system(),
        }
    }

    #[cfg(test)]
    fn with_roots(state_root: PathBuf, install_root: PathBuf) -> Self {
        Self {
            paths: ApoPaths::from_roots(state_root, install_root),
        }
    }

    pub fn probe(&self) -> BackendProbe {
        if let Some(component) = self.missing_component() {
            return BackendProbe {
                readiness: RelayReadiness::ComponentRequired,
                physical_output: None,
                detail: Some(format!(
                    "Voxveil system-audio component is incomplete: {component} is missing"
                )),
            };
        }

        match self.runtime_heartbeat() {
            Ok(heartbeat) if heartbeat.process_count > 0 && heartbeat_is_fresh(heartbeat.timestamp_ms) => {
                BackendProbe {
                    readiness: RelayReadiness::Ready,
                    physical_output: None,
                    detail: None,
                }
            }
            Ok(heartbeat) if heartbeat.process_count == 0 => BackendProbe {
                readiness: RelayReadiness::Faulted,
                physical_output: None,
                detail: Some(
                    "Voxveil is installed, but Windows has not sent audio through the APO yet. Play audio on the installed output device, then refocus Voxveil."
                        .into(),
                ),
            },
            Ok(_) => BackendProbe {
                readiness: RelayReadiness::Faulted,
                physical_output: None,
                detail: Some(
                    "Voxveil is installed, but the APO processing heartbeat is stale. Restart the audio device or Windows."
                        .into(),
                ),
            },
            Err(_) => BackendProbe {
                readiness: RelayReadiness::Faulted,
                physical_output: None,
                detail: Some(
                    "Voxveil is installed, but Windows has not loaded the APO on the audio graph yet. Play audio or restart Windows, then refocus Voxveil."
                        .into(),
                ),
            },
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, vocal_level: u8) -> Result<BackendProbe, String> {
        let probe = self.probe();
        if probe.readiness != RelayReadiness::Ready {
            return Err(probe
                .detail
                .clone()
                .unwrap_or_else(|| "Voxveil APO is unavailable".into()));
        }
        self.write_control(enabled, vocal_level)?;
        Ok(probe)
    }

    pub fn set_vocal_level(&self, value: u8) {
        if self.probe().readiness != RelayReadiness::Ready {
            return;
        }
        let enabled = self
            .read_control()
            .map(|bytes| bytes[1] != 0)
            .unwrap_or(false);
        let _ = self.write_control(enabled, value);
    }

    fn missing_component(&self) -> Option<&'static str> {
        [
            (&self.paths.marker, "installation marker"),
            (&self.paths.dll, "VoxveilApo.dll"),
            (&self.paths.control, "APO control file"),
        ]
        .into_iter()
        .find_map(|(path, label)| (!path.is_file()).then_some(label))
    }

    fn runtime_heartbeat(&self) -> Result<RuntimeHeartbeat, String> {
        let bytes = fs::read(&self.paths.runtime).map_err(|error| error.to_string())?;
        parse_runtime_heartbeat(&bytes)
    }

    fn read_control(&self) -> Result<[u8; 3], String> {
        let bytes = fs::read(&self.paths.control).map_err(|error| error.to_string())?;
        if bytes.len() != 3 || bytes[0] != CONTROL_VERSION {
            return Err("Voxveil APO control file has an unsupported format".into());
        }
        Ok([bytes[0], bytes[1], bytes[2]])
    }

    fn write_control(&self, enabled: bool, vocal_level: u8) -> Result<(), String> {
        let bytes = [CONTROL_VERSION, u8::from(enabled), vocal_level.min(100)];
        fs::write(&self.paths.control, bytes).map_err(|error| {
            format!(
                "failed to update Voxveil APO control file {}: {error}",
                display_path(&self.paths.control)
            )
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeHeartbeat {
    timestamp_ms: u64,
    process_count: u64,
    _pid: u32,
}

fn parse_runtime_heartbeat(bytes: &[u8]) -> Result<RuntimeHeartbeat, String> {
    if bytes.len() != RUNTIME_SIZE || bytes[0] != RUNTIME_VERSION {
        return Err("Voxveil APO runtime heartbeat has an unsupported format".into());
    }
    let timestamp_ms = u64::from_le_bytes(bytes[1..9].try_into().expect("heartbeat timestamp"));
    let process_count = u64::from_le_bytes(bytes[9..17].try_into().expect("heartbeat count"));
    let pid = u32::from_le_bytes(bytes[17..21].try_into().expect("heartbeat pid"));
    Ok(RuntimeHeartbeat {
        timestamp_ms,
        process_count,
        _pid: pid,
    })
}

fn heartbeat_is_fresh(timestamp_ms: u64) -> bool {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    now_ms.saturating_sub(timestamp_ms) <= HEARTBEAT_MAX_AGE_MS
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl Default for ApoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestRoots {
        root: PathBuf,
        state: PathBuf,
        install: PathBuf,
    }

    impl TestRoots {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("voxveil-apo-{name}-{}-{nonce}", std::process::id()));
            let state = root.join("state");
            let install = root.join("install");
            fs::create_dir_all(&state).expect("state root");
            fs::create_dir_all(install.join("system-audio")).expect("install root");
            Self {
                root,
                state,
                install,
            }
        }

        fn install_component(&self, control: [u8; 3], process_count: u64) {
            fs::write(self.state.join("apo-installed.json"), b"{}").expect("marker");
            fs::write(self.state.join("apo-control.bin"), control).expect("control");
            fs::write(
                self.install.join("system-audio").join("VoxveilApo.dll"),
                b"dll",
            )
            .expect("dll");
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_millis() as u64;
            let mut heartbeat = Vec::with_capacity(RUNTIME_SIZE);
            heartbeat.push(RUNTIME_VERSION);
            heartbeat.extend_from_slice(&now.to_le_bytes());
            heartbeat.extend_from_slice(&process_count.to_le_bytes());
            heartbeat.extend_from_slice(&1234_u32.to_le_bytes());
            fs::write(self.state.join("apo-runtime.bin"), heartbeat).expect("runtime");
        }
    }

    impl Drop for TestRoots {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn missing_component_never_claims_ready() {
        let roots = TestRoots::new("missing");
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        assert_eq!(backend.probe().readiness, RelayReadiness::ComponentRequired);
    }

    #[test]
    fn complete_installation_is_ready_after_audio_was_processed() {
        let roots = TestRoots::new("ready");
        roots.install_component([1, 0, 100], 1);
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        assert_eq!(backend.probe().readiness, RelayReadiness::Ready);
    }

    #[test]
    fn installed_component_without_processed_audio_is_not_ready() {
        let roots = TestRoots::new("inactive");
        roots.install_component([1, 0, 100], 0);
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        assert_eq!(backend.probe().readiness, RelayReadiness::Faulted);
    }

    #[test]
    fn enabling_updates_control_file() {
        let roots = TestRoots::new("enable");
        roots.install_component([1, 0, 100], 1);
        let mut backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        backend.set_enabled(true, 25).expect("enable APO");
        assert_eq!(
            fs::read(roots.state.join("apo-control.bin")).unwrap(),
            [1, 1, 25]
        );
    }

    #[test]
    fn vocal_level_preserves_enabled_state_and_clamps() {
        let roots = TestRoots::new("level");
        roots.install_component([1, 1, 40], 1);
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        backend.set_vocal_level(250);
        assert_eq!(
            fs::read(roots.state.join("apo-control.bin")).unwrap(),
            [1, 1, 100]
        );
    }
}
