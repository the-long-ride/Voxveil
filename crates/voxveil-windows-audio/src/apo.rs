use crate::{BackendProbe, RelayReadiness};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const CONTROL_VERSION: u8 = 1;

#[derive(Clone, Debug)]
struct ApoPaths {
    marker: PathBuf,
    control: PathBuf,
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
        let missing = self.missing_component();
        if let Some(component) = missing {
            return BackendProbe {
                readiness: RelayReadiness::ComponentRequired,
                physical_output: None,
                detail: Some(format!(
                    "Voxveil system-audio component is incomplete: {component} is missing"
                )),
            };
        }

        BackendProbe {
            readiness: RelayReadiness::Ready,
            physical_output: None,
            detail: None,
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

        fn install_component(&self, control: [u8; 3]) {
            fs::write(self.state.join("apo-installed.json"), b"{}").expect("marker");
            fs::write(self.state.join("apo-control.bin"), control).expect("control");
            fs::write(
                self.install.join("system-audio").join("VoxveilApo.dll"),
                b"dll",
            )
            .expect("dll");
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
    fn complete_installation_is_ready() {
        let roots = TestRoots::new("ready");
        roots.install_component([1, 0, 100]);
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        assert_eq!(backend.probe().readiness, RelayReadiness::Ready);
    }

    #[test]
    fn enabling_updates_control_file() {
        let roots = TestRoots::new("enable");
        roots.install_component([1, 0, 100]);
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
        roots.install_component([1, 1, 40]);
        let backend = ApoBackend::with_roots(roots.state.clone(), roots.install.clone());
        backend.set_vocal_level(250);
        assert_eq!(
            fs::read(roots.state.join("apo-control.bin")).unwrap(),
            [1, 1, 100]
        );
    }
}
