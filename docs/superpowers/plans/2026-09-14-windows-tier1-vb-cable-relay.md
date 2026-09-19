# Windows Tier 1 VB-CABLE Relay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a production-usable Windows all-output relay that uses an already-installed standard VB-CABLE render endpoint, applies the existing Voxveil DSP, and renders to a safe physical endpoint without TESTSIGNING.

**Architecture:** Keep the loaded componentized APO as highest runtime priority. When no APO is loaded, detect the standard VB-CABLE render endpoint, require it to be the current Windows default render endpoint, then run a dedicated WASAPI worker that loopback-captures the cable, processes stereo float samples through Voxveil DSP, and renders them to a separately selected physical endpoint using shared-mode format conversion. Tauri exposes strategy-neutral status, fixed safe helper actions, and physical-output selection; React presents the setup flow.

**Tech Stack:** Rust, `wasapi = 0.23.0`, `windows = 0.62.2`, Tauri, React/TypeScript, Vitest, PowerShell, existing `voxveil-audio-core`/`voxveil-dsp`.

**Spec:** `docs/superpowers/specs/2026-09-14-windows-signed-audio-paths-design.md`

## Global Constraints

- An already loaded Voxveil APO remains the preferred active backend.
- Voxveil does not bundle, redistribute, silently download, or silently install VB-CABLE.
- Tier 1 supports only the standard VB-CABLE endpoint; A/B/C/D variants are excluded.
- Do not mutate Windows default devices through undocumented `PolicyConfig` interfaces.
- `ready` means interception, capture/DSP, and physical rendering are actually active.
- Never render the relay back into VB-CABLE or another known Voxveil virtual endpoint.
- Persist endpoint IDs, not only display names.
- Do not persist raw audio.
- All new user-visible strings go through the existing localization system.
- Keep non-Windows builds compiling and behaviorally unchanged.
- Do not restore GitHub Actions; the repository remains workflow-free.
- Fixed external actions may open only Windows Sound settings and the official `https://vb-audio.com/Cable/` page.

---

### Task 1: Introduce strategy-neutral Windows route types

**Files:**
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Produces: `WindowsInterceptionKind`, `WindowsAudioRoute`, expanded `BackendProbe`.
- Consumes: existing `RelayReadiness` and `EndpointDescriptor`.

- [ ] **Step 1: Write failing tests for the new route model**

Add tests in `device.rs` that construct APO and relay routes and assert route identity is preserved:

```rust
#[test]
fn backend_probe_carries_interception_route() {
    let probe = BackendProbe {
        readiness: RelayReadiness::Ready,
        route: WindowsAudioRoute {
            interception: Some(WindowsInterceptionKind::VbCableRelay),
            source_endpoint_id: Some("cable".into()),
            source_display_name: Some("CABLE Input (VB-Audio Virtual Cable)".into()),
            physical_output_endpoint_id: Some("speakers".into()),
            physical_output_display_name: Some("Speakers".into()),
        },
        detail: None,
    };
    assert_eq!(probe.route.interception, Some(WindowsInterceptionKind::VbCableRelay));
    assert_eq!(probe.route.physical_output_endpoint_id.as_deref(), Some("speakers"));
}
```

- [ ] **Step 2: Run the crate tests and confirm failure**

Run:

```powershell
cargo test -p voxveil-windows-audio device::tests::backend_probe_carries_interception_route
```

Expected: compile failure because `WindowsInterceptionKind`, `WindowsAudioRoute`, and `BackendProbe::route` do not exist.

- [ ] **Step 3: Add the route types and migrate `BackendProbe`**

Add:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsInterceptionKind {
    Apo,
    VbCableRelay,
    VoxveilCableRelay,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowsAudioRoute {
    pub interception: Option<WindowsInterceptionKind>,
    pub source_endpoint_id: Option<String>,
    pub source_display_name: Option<String>,
    pub physical_output_endpoint_id: Option<String>,
    pub physical_output_display_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendProbe {
    pub readiness: RelayReadiness,
    pub route: WindowsAudioRoute,
    pub detail: Option<String>,
}
```

Update `BackendProbe::unsupported()` and `component_probe()` so APO-ready probes set `interception: Some(WindowsInterceptionKind::Apo)` and physical output display information is stored inside `route`.

Re-export the two new public types from `lib.rs`.

- [ ] **Step 4: Run the full crate tests**

Run:

```powershell
cargo test -p voxveil-windows-audio
```

Expected: PASS after all old `physical_output` field references are migrated to `probe.route.physical_output_display_name`.

- [ ] **Step 5: Commit**

```bash
git add crates/voxveil-windows-audio/src/device.rs crates/voxveil-windows-audio/src/lib.rs
git commit -m "refactor(windows): model interception routes"
```

### Task 2: Enrich render endpoint metadata and classify VB-CABLE safely

**Files:**
- Create: `crates/voxveil-windows-audio/src/virtual_endpoint.rs`
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- Produces: `VirtualEndpointKind`, `classify_virtual_endpoint(&EndpointDescriptor)`, `find_standard_vb_cable(&[EndpointDescriptor])`.
- `EndpointDescriptor` gains optional `interface_name` and `description` metadata.

- [ ] **Step 1: Write classifier tests**

Create `virtual_endpoint.rs` with tests first:

```rust
#[test]
fn recognizes_standard_vb_cable_name() {
    let endpoint = endpoint("cable", "CABLE Input (VB-Audio Virtual Cable)", "VB-Audio Virtual Cable", "CABLE Input");
    assert_eq!(classify_virtual_endpoint(&endpoint), Some(VirtualEndpointKind::VbCable));
}

#[test]
fn rejects_generic_cable_named_device() {
    let endpoint = endpoint("other", "My CABLE Input", "USB Audio", "Speakers");
    assert_eq!(classify_virtual_endpoint(&endpoint), None);
}

#[test]
fn ignores_vb_cable_capture_name_on_render_list() {
    let endpoint = endpoint("capture-name", "CABLE Output (VB-Audio Virtual Cable)", "VB-Audio Virtual Cable", "CABLE Output");
    assert_eq!(classify_virtual_endpoint(&endpoint), None);
}
```

Use this helper only in tests:

```rust
fn endpoint(id: &str, name: &str, interface_name: &str, description: &str) -> EndpointDescriptor {
    EndpointDescriptor {
        id: id.into(),
        name: name.into(),
        interface_name: Some(interface_name.into()),
        description: Some(description.into()),
        is_default: false,
    }
}
```

- [ ] **Step 2: Run the classifier test and confirm failure**

```powershell
cargo test -p voxveil-windows-audio virtual_endpoint::tests
```

Expected: compile failure because the module and metadata fields do not exist.

- [ ] **Step 3: Implement fail-closed classification**

Implement canonical-name matching only:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualEndpointKind {
    VbCable,
    VoxveilCable,
}

pub(crate) fn classify_virtual_endpoint(endpoint: &EndpointDescriptor) -> Option<VirtualEndpointKind> {
    let name = endpoint.name.trim();
    let description = endpoint.description.as_deref().unwrap_or("").trim();
    let interface_name = endpoint.interface_name.as_deref().unwrap_or("").trim();

    let standard_name = name.eq_ignore_ascii_case("CABLE Input")
        || name.eq_ignore_ascii_case("CABLE Input (VB-Audio Virtual Cable)");
    let metadata_matches = description.eq_ignore_ascii_case("CABLE Input")
        || interface_name.to_ascii_lowercase().contains("vb-audio virtual cable")
        || name.to_ascii_lowercase().contains("vb-audio virtual cable");

    (standard_name && metadata_matches).then_some(VirtualEndpointKind::VbCable)
}

pub(crate) fn find_standard_vb_cable(endpoints: &[EndpointDescriptor]) -> Option<&EndpointDescriptor> {
    endpoints.iter().find(|endpoint| classify_virtual_endpoint(endpoint) == Some(VirtualEndpointKind::VbCable))
}
```

Do not add A/B/C/D aliases.

- [ ] **Step 4: Populate endpoint metadata during WASAPI enumeration**

In `enumerate_render_inner()` collect:

```rust
let interface_name = device.get_interface_friendlyname().ok();
let description = device.get_description().ok();
endpoints.push(EndpointDescriptor {
    id,
    name,
    interface_name,
    description,
    is_default: id == default_id,
});
```

- [ ] **Step 5: Run the crate tests**

```powershell
cargo test -p voxveil-windows-audio
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/voxveil-windows-audio/src/device.rs crates/voxveil-windows-audio/src/lib.rs crates/voxveil-windows-audio/src/relay.rs crates/voxveil-windows-audio/src/virtual_endpoint.rs
git commit -m "feat(windows): detect standard VB-CABLE endpoint"
```

### Task 3: Add deterministic physical-output selection and feedback prevention

**Files:**
- Create: `crates/voxveil-windows-audio/src/route.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Produces: `PhysicalOutputSelection`, `select_physical_output(endpoints, virtual_id, preferred_id)`.
- Consumes: `classify_virtual_endpoint` and `EndpointDescriptor`.

- [ ] **Step 1: Write route-selection tests**

```rust
#[test]
fn preferred_physical_endpoint_wins_when_safe() {
    let endpoints = vec![physical("speakers", "Speakers", false), physical("headphones", "Headphones", true)];
    let selected = select_physical_output(&endpoints, "cable", Some("speakers")).unwrap();
    assert_eq!(selected.id, "speakers");
}

#[test]
fn non_virtual_default_is_fallback() {
    let endpoints = vec![physical("speakers", "Speakers", true)];
    let selected = select_physical_output(&endpoints, "cable", Some("gone")).unwrap();
    assert_eq!(selected.id, "speakers");
}

#[test]
fn selected_virtual_endpoint_is_never_physical_output() {
    let endpoints = vec![vb_cable("cable", true)];
    assert!(select_physical_output(&endpoints, "cable", Some("cable")).is_none());
}
```

- [ ] **Step 2: Run tests and confirm failure**

```powershell
cargo test -p voxveil-windows-audio route::tests
```

Expected: compile failure because route selection does not exist.

- [ ] **Step 3: Implement selection using endpoint IDs**

```rust
pub(crate) fn select_physical_output<'a>(
    endpoints: &'a [EndpointDescriptor],
    virtual_endpoint_id: &str,
    preferred_id: Option<&str>,
) -> Option<&'a EndpointDescriptor> {
    let safe = |endpoint: &&EndpointDescriptor| {
        endpoint.id != virtual_endpoint_id
            && classify_virtual_endpoint(endpoint).is_none()
    };

    preferred_id
        .and_then(|id| endpoints.iter().find(|endpoint| endpoint.id == id).filter(safe))
        .or_else(|| endpoints.iter().find(|endpoint| endpoint.is_default).filter(safe))
        .or_else(|| endpoints.iter().find(safe))
}
```

Also add a direct identity guard used immediately before relay startup:

```rust
pub(crate) fn validate_distinct_route(source_id: &str, physical_id: &str) -> Result<(), &'static str> {
    (source_id != physical_id)
        .then_some(())
        .ok_or("virtual interception endpoint cannot also be the physical output")
}
```

- [ ] **Step 4: Run tests**

```powershell
cargo test -p voxveil-windows-audio route::tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/voxveil-windows-audio/src/route.rs crates/voxveil-windows-audio/src/lib.rs
git commit -m "feat(windows): select safe physical relay output"
```

### Task 4: Add a testable relay lifecycle abstraction

**Files:**
- Create: `crates/voxveil-windows-audio/src/relay_engine.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Produces: `RelaySpec`, `RelayRuntimeState`, `RelayCommand`, `RelayHandle`.
- Later WASAPI code implements `run_relay_worker(spec, vocal_level, control_rx, state_tx)`.

- [ ] **Step 1: Write state-machine tests**

Use a fake worker function to prove lifecycle semantics without audio hardware:

```rust
#[test]
fn running_state_is_required_for_ready() {
    assert!(!RelayRuntimeState::Starting.is_running());
    assert!(RelayRuntimeState::Running.is_running());
    assert!(!RelayRuntimeState::Faulted("device removed".into()).is_running());
}

#[test]
fn route_identity_collision_is_rejected_before_spawn() {
    let spec = RelaySpec {
        source_endpoint_id: "same".into(),
        physical_output_endpoint_id: "same".into(),
    };
    assert!(RelayHandle::validate_spec(&spec).is_err());
}
```

- [ ] **Step 2: Run tests and confirm failure**

```powershell
cargo test -p voxveil-windows-audio relay_engine::tests
```

Expected: compile failure.

- [ ] **Step 3: Implement lifecycle types**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelaySpec {
    pub source_endpoint_id: String,
    pub physical_output_endpoint_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RelayRuntimeState {
    Stopped,
    Starting,
    Running,
    Faulted(String),
}

impl RelayRuntimeState {
    pub(crate) fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

pub(crate) enum RelayCommand {
    SetVocalLevel(u8),
    Stop,
}
```

`RelayHandle` owns `std::sync::mpsc::Sender<RelayCommand>`, the worker `JoinHandle`, and a shared latest `RelayRuntimeState`. `stop()` sends `Stop` and joins the worker. `Drop` calls the same stop path without panicking.

- [ ] **Step 4: Run tests**

```powershell
cargo test -p voxveil-windows-audio relay_engine::tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/voxveil-windows-audio/src/relay_engine.rs crates/voxveil-windows-audio/src/lib.rs
git commit -m "feat(windows): add relay lifecycle state machine"
```

### Task 5: Implement the WASAPI loopback-to-physical worker

**Files:**
- Create: `crates/voxveil-windows-audio/src/wasapi_relay.rs`
- Modify: `crates/voxveil-windows-audio/src/relay_engine.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Test: `crates/voxveil-windows-audio/src/sample.rs`

**Interfaces:**
- Produces: `run_relay_worker(RelaySpec, u8, Receiver<RelayCommand>, Arc<Mutex<RelayRuntimeState>>) -> Result<(), String>`.
- Consumes: `process_f32le_stereo`, `wasapi::DeviceEnumerator`, `Direction`, `StreamMode`.

- [ ] **Step 1: Add format-policy tests before hardware code**

Add pure tests for the only accepted DSP input format:

```rust
#[test]
fn relay_dsp_requires_stereo_f32() {
    assert!(relay_format_supported(2, 32, true));
    assert!(!relay_format_supported(1, 32, true));
    assert!(!relay_format_supported(2, 16, false));
}
```

Implement the pure helper in `wasapi_relay.rs` as:

```rust
fn relay_format_supported(channels: u16, bits: u16, is_float: bool) -> bool {
    channels == 2 && bits == 32 && is_float
}
```

- [ ] **Step 2: Run the pure test and confirm failure, then make it pass**

```powershell
cargo test -p voxveil-windows-audio wasapi_relay::tests::relay_dsp_requires_stereo_f32
```

Expected before helper: compile failure. Expected after helper: PASS.

- [ ] **Step 3: Resolve both endpoint IDs inside the worker thread**

The worker must initialize COM itself and never move `DeviceEnumerator` across threads:

```rust
wasapi::initialize_mta().ok().map_err(|error| error.to_string())?;
let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
let source_device = enumerator.get_device(&spec.source_endpoint_id).map_err(|error| error.to_string())?;
let physical_device = enumerator.get_device(&spec.physical_output_endpoint_id).map_err(|error| error.to_string())?;
```

Guarantee `wasapi::deinitialize()` executes on every worker exit through a small RAII guard local to this module.

- [ ] **Step 4: Initialize loopback capture from the virtual render endpoint**

Use the source render endpoint's mix format and shared-mode loopback behavior:

```rust
let mut capture_client = source_device.get_iaudioclient().map_err(|error| error.to_string())?;
let source_format = capture_client.get_mixformat().map_err(|error| error.to_string())?;
let sample_type = source_format.get_subformat().map_err(|error| error.to_string())?;
if !relay_format_supported(
    source_format.get_nchannels(),
    source_format.get_bitspersample(),
    sample_type == wasapi::SampleType::Float,
) {
    return Err("VB-CABLE shared format must be stereo 32-bit float for this relay version".into());
}
let stream_mode = StreamMode::PollingShared {
    autoconvert: false,
    buffer_duration_hns: 200_000,
};
capture_client
    .initialize_client(&source_format, &Direction::Capture, &stream_mode)
    .map_err(|error| error.to_string())?;
let capture = capture_client.get_audiocaptureclient().map_err(|error| error.to_string())?;
```

With a render `Device` initialized for `Direction::Capture`, `wasapi` sets WASAPI's loopback flag internally.

- [ ] **Step 5: Initialize shared-mode physical rendering with Windows format conversion**

```rust
let mut render_client = physical_device.get_iaudioclient().map_err(|error| error.to_string())?;
let render_mode = StreamMode::PollingShared {
    autoconvert: true,
    buffer_duration_hns: 200_000,
};
render_client
    .initialize_client(&source_format, &Direction::Render, &render_mode)
    .map_err(|error| error.to_string())?;
let render = render_client.get_audiorenderclient().map_err(|error| error.to_string())?;
```

This deliberately lets the Windows shared-mode audio engine handle source/physical sample-rate conversion.

- [ ] **Step 6: Implement the polling loop with bounded buffering**

Use a single worker thread and `VecDeque<u8>` so COM/audio objects remain thread-local:

```rust
let bytes_per_frame = source_format.get_blockalign() as usize;
let mut queue = std::collections::VecDeque::<u8>::new();
let mut capture_buffer = vec![0_u8; bytes_per_frame * 4096];
let mut vocal_level = initial_vocal_level.min(100);

render_client.start_stream().map_err(|error| error.to_string())?;
capture_client.start_stream().map_err(|error| error.to_string())?;
set_state(&state, RelayRuntimeState::Running);

loop {
    while let Ok(command) = control_rx.try_recv() {
        match command {
            RelayCommand::SetVocalLevel(value) => vocal_level = value.min(100),
            RelayCommand::Stop => return Ok(()),
        }
    }

    if let Some(frames) = capture.get_next_packet_size().map_err(|error| error.to_string())? {
        if frames > 0 {
            let bytes = frames as usize * bytes_per_frame;
            if capture_buffer.len() < bytes {
                capture_buffer.resize(bytes, 0);
            }
            let (read_frames, info) = capture
                .read_from_device(&mut capture_buffer[..bytes])
                .map_err(|error| error.to_string())?;
            let read_bytes = read_frames as usize * bytes_per_frame;
            if info.flags.silent {
                capture_buffer[..read_bytes].fill(0);
            } else {
                process_f32le_stereo(&mut capture_buffer[..read_bytes], vocal_level)
                    .map_err(str::to_string)?;
            }
            queue.extend(&capture_buffer[..read_bytes]);
        }
    }

    let writable = render_client
        .get_available_space_in_frames()
        .map_err(|error| error.to_string())? as usize;
    let queued_frames = queue.len() / bytes_per_frame;
    let frames_to_write = writable.min(queued_frames);
    if frames_to_write > 0 {
        render
            .write_to_device_from_deque(frames_to_write, &mut queue, None)
            .map_err(|error| error.to_string())?;
    }

    if queue.len() > bytes_per_frame * 8192 {
        return Err("relay buffer exceeded the bounded latency limit".into());
    }
    std::thread::sleep(std::time::Duration::from_millis(2));
}
```

If the exact `BufferInfo` field accessor differs in `wasapi 0.23.0`, use its public silent flag/accessor while preserving this behavior; do not infer silence from zero bytes.

- [ ] **Step 7: Stop streams and publish faults deterministically**

On normal stop, stop capture then render. On error, write `RelayRuntimeState::Faulted(error.clone())` before worker exit. Do not mark `Running` before both clients are initialized and started.

- [ ] **Step 8: Run crate tests and Windows compile check**

```powershell
cargo test -p voxveil-windows-audio
cargo check -p voxveil-windows-audio --target x86_64-pc-windows-msvc
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/voxveil-windows-audio/src/wasapi_relay.rs crates/voxveil-windows-audio/src/relay_engine.rs crates/voxveil-windows-audio/src/lib.rs crates/voxveil-windows-audio/src/sample.rs
git commit -m "feat(windows): relay VB-CABLE through WASAPI DSP"
```

### Task 6: Refactor `WindowsAudioBackend` to arbitrate APO and VB-CABLE

**Files:**
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Test: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- `WindowsAudioBackend::probe()` chooses APO first, then VB-CABLE.
- Adds `set_physical_output(&mut self, endpoint_id: Option<String>) -> Result<BackendProbe, String>`.
- Owns `Option<RelayHandle>` and `preferred_physical_output_id`.

- [ ] **Step 1: Write pure backend-decision tests**

Extract a pure decision helper and test precedence:

```rust
#[test]
fn loaded_apo_has_priority_over_cable() {
    let decision = decide_backend(true, Some(cable(true)), Some(physical("speakers", false)), false);
    assert_eq!(decision.interception, Some(WindowsInterceptionKind::Apo));
    assert_eq!(decision.readiness, RelayReadiness::Ready);
}

#[test]
fn missing_cable_requires_component_when_apo_not_loaded() {
    let decision = decide_backend(false, None, Some(physical("speakers", true)), false);
    assert_eq!(decision.readiness, RelayReadiness::ComponentRequired);
}

#[test]
fn installed_cable_must_be_default_for_all_output_capture() {
    let decision = decide_backend(false, Some(cable(false)), Some(physical("speakers", true)), false);
    assert_eq!(decision.readiness, RelayReadiness::RoutingRequired);
}

#[test]
fn cable_route_is_ready_only_when_worker_is_running() {
    let decision = decide_backend(false, Some(cable(true)), Some(physical("speakers", false)), true);
    assert_eq!(decision.readiness, RelayReadiness::Ready);
}
```

- [ ] **Step 2: Run tests and confirm failure**

```powershell
cargo test -p voxveil-windows-audio relay::tests
```

Expected: compile failure for `decide_backend`.

- [ ] **Step 3: Implement decision semantics**

Rules in code:

```rust
if apo_loaded {
    // Ready/Apo; stop an existing relay because the native path now owns processing.
} else if cable.is_none() {
    // ComponentRequired with official-dependency detail.
} else if !cable.unwrap().is_default {
    // RoutingRequired: CABLE Input must be Windows default render endpoint.
} else if physical.is_none() {
    // RoutingRequired: no safe physical destination.
} else if relay_running {
    // Ready/VbCableRelay.
} else {
    // RoutingRequired before enable; Faulted when an existing worker reports Faulted.
}
```

- [ ] **Step 4: Start/stop the relay from `set_enabled()`**

`set_enabled(true, vocal_level)` must:

1. re-enumerate endpoints;
2. use APO commands when APO is loaded;
3. otherwise validate cable is default and choose physical endpoint;
4. create `RelaySpec` and start `RelayHandle`;
5. wait only for a bounded startup state transition, maximum 2 seconds;
6. return `Ready` only if worker state becomes `Running`.

`set_enabled(false, ...)` must stop the relay if present and disable the APO if the control executable exists.

- [ ] **Step 5: Route live vocal changes to the active strategy**

```rust
pub fn set_vocal_level(&mut self, value: u8) {
    self.vocal_level = value.min(100);
    if let Some(relay) = &self.relay {
        relay.set_vocal_level(self.vocal_level);
    }
    if let Some(control) = control_executable() {
        let percent = self.vocal_level.to_string();
        let _ = run_control(&control, &["vocal", percent.as_str()]);
    }
}
```

- [ ] **Step 6: Return endpoint IDs from physical-output enumeration**

Replace the string-only helper with:

```rust
pub fn physical_outputs(&self) -> Vec<EndpointDescriptor> {
    enumerate_render_blocking()
        .map(|items| items.into_iter().filter(|item| classify_virtual_endpoint(item).is_none()).collect())
        .unwrap_or_default()
}
```

- [ ] **Step 7: Run tests**

```powershell
cargo test -p voxveil-windows-audio
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/voxveil-windows-audio/src/relay.rs crates/voxveil-windows-audio/src/device.rs
git commit -m "feat(windows): arbitrate APO and VB-CABLE backends"
```

### Task 7: Persist the selected physical endpoint ID

**Files:**
- Create: `tauri/config/windows_audio.rs`
- Modify: `tauri/config/mod.rs`
- Modify: `tauri/platform/controller.rs`
- Modify: `tauri/lib.rs`
- Modify: `tauri/Cargo.toml` only if serde JSON is not already available there.

**Interfaces:**
- Produces: `WindowsAudioPreferences { physical_output_endpoint_id, preferred_interception }` load/save helpers.
- `ProcessingController::set_physical_output(Option<String>)` passes the endpoint ID into `WindowsAudioBackend`.

- [ ] **Step 1: Write serialization/path-independent tests**

```rust
#[test]
fn windows_audio_preferences_round_trip() {
    let prefs = WindowsAudioPreferences {
        physical_output_endpoint_id: Some("speakers-id".into()),
        preferred_interception: None,
    };
    let json = serde_json::to_string(&prefs).unwrap();
    assert_eq!(serde_json::from_str::<WindowsAudioPreferences>(&json).unwrap(), prefs);
}
```

- [ ] **Step 2: Implement preferences without storing audio data**

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsAudioPreferences {
    pub physical_output_endpoint_id: Option<String>,
    pub preferred_interception: Option<String>,
}
```

Use `AppHandle::path().app_config_dir()` and store exactly `windows-audio.json`. Save atomically through `windows-audio.json.tmp` + rename. The JSON contains only endpoint IDs/strategy strings.

- [ ] **Step 3: Restore preferences during Tauri setup**

In `tauri/lib.rs`, add a setup hook after `.manage(controller)`:

```rust
.setup(|app| {
    #[cfg(target_os = "windows")]
    {
        let prefs = config::windows_audio::load(app.handle())?;
        if let Some(endpoint_id) = prefs.physical_output_endpoint_id {
            app.state::<platform::ProcessingController>()
                .set_physical_output(Some(endpoint_id))?;
        }
    }
    Ok(())
})
```

Import `tauri::Manager` as required.

- [ ] **Step 4: Run Rust tests**

```powershell
cargo test -p voxveil-tauri
cargo test -p voxveil-windows-audio
```

If the Tauri crate package name differs, use the package name declared in `tauri/Cargo.toml` for the first command.

- [ ] **Step 5: Commit**

```bash
git add tauri/config/windows_audio.rs tauri/config/mod.rs tauri/platform/controller.rs tauri/lib.rs tauri/Cargo.toml
git commit -m "feat(windows): persist physical audio route"
```

### Task 8: Expose route status, physical-output selection, and fixed helper actions through Tauri

**Files:**
- Modify: `tauri/app/dto.rs`
- Modify: `tauri/app/commands.rs`
- Modify: `tauri/app/system_audio.rs`
- Modify: `tauri/platform/controller.rs`
- Modify: `tauri/lib.rs`

**Interfaces:**
- Produces UI DTO fields `interceptionKind`, `physicalOutputEndpointId`.
- Produces commands `set_physical_audio_output`, `open_windows_sound_settings`, `open_vb_cable_download`.

- [ ] **Step 1: Add DTO mapping tests**

Add a pure mapping test for:

```rust
fn interception_name(kind: WindowsInterceptionKind) -> &'static str {
    match kind {
        WindowsInterceptionKind::Apo => "apo",
        WindowsInterceptionKind::VbCableRelay => "vb-cable-relay",
        WindowsInterceptionKind::VoxveilCableRelay => "voxveil-cable-relay",
    }
}
```

Expected strings are part of the UI API and must not use Rust debug formatting.

- [ ] **Step 2: Expand `BackendSnapshot` and `AppViewState`**

Add:

```rust
pub backend_kind: Option<String>,
pub physical_output_endpoint_id: Option<String>,
```

Populate them from `BackendProbe.route` while keeping `physical_output` as the human-readable name.

- [ ] **Step 3: Change `list_audio_outputs` to stable DTOs**

Define:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutputDto {
    pub endpoint_id: String,
    pub display_name: String,
    pub is_default: bool,
}
```

Return `Vec<AudioOutputDto>` built from the filtered physical `EndpointDescriptor` list.

- [ ] **Step 4: Add physical-output selection command**

```rust
#[tauri::command]
pub fn set_physical_audio_output(
    app: AppHandle,
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    endpoint_id: String,
) -> Result<(), String> {
    let snapshot = controller.set_physical_output(Some(endpoint_id.clone()))?;
    let mut prefs = crate::config::windows_audio::load(&app)?;
    prefs.physical_output_endpoint_id = Some(endpoint_id);
    crate::config::windows_audio::save(&app, &prefs)?;
    state.lock()?.apply_backend(&snapshot);
    Ok(())
}
```

Validate the selected endpoint exists and is non-virtual inside the controller/backend before persisting.

- [ ] **Step 5: Add fixed helper commands with no user-supplied target**

On Windows:

```rust
#[tauri::command]
pub fn open_windows_sound_settings() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("ms-settings:sound")
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open Windows Sound settings: {error}"))
}

#[tauri::command]
pub fn open_vb_cable_download() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("https://vb-audio.com/Cable/")
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open the VB-CABLE website: {error}"))
}
```

On non-Windows builds, return a platform-unavailable error. Do not accept arbitrary URLs or shell fragments.

- [ ] **Step 6: Register commands and run tests**

```powershell
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add tauri/app/dto.rs tauri/app/commands.rs tauri/app/system_audio.rs tauri/platform/controller.rs tauri/lib.rs
git commit -m "feat(windows): expose relay routing controls"
```

### Task 9: Add the VB-CABLE onboarding and physical-output UI

**Files:**
- Modify: `ui/lib/types.ts`
- Modify: `ui/lib/tauri.ts`
- Modify: `ui/app/useVoxveilState.ts`
- Modify: `ui/features/home/HomeScreen.tsx`
- Modify: `ui/features/home/SystemAudioEndpoints.tsx`
- Modify: `ui/features/home/SystemAudioEndpoints.test.tsx`
- Modify: `locales/en/system-audio.json`
- Modify: `locales/vi/system-audio.json`
- Modify: `locales/zh/system-audio.json`
- Modify: `locales/ko/system-audio.json`
- Modify: `locales/ja/system-audio.json`
- Modify: `locales/es/system-audio.json`
- Modify: `locales/fr/system-audio.json`

**Interfaces:**
- UI stable backend type: `'apo' | 'vb-cable-relay' | 'voxveil-cable-relay' | null`.
- UI physical output: `{ endpointId, displayName, isDefault }`.

- [ ] **Step 1: Extend TypeScript types**

```ts
export type WindowsInterceptionKind = 'apo' | 'vb-cable-relay' | 'voxveil-cable-relay';

export interface AudioOutput {
  endpointId: string;
  displayName: string;
  isDefault: boolean;
}
```

Add to `VoxveilState`:

```ts
backendKind: WindowsInterceptionKind | null;
physicalOutputEndpointId: string | null;
```

- [ ] **Step 2: Write UI tests for missing/routing/ready states**

Add tests that render the panel and assert:

```ts
expect(screen.getByRole('button', { name: /get vb-cable/i })).toBeInTheDocument();
expect(screen.getByRole('button', { name: /open sound settings/i })).toBeInTheDocument();
expect(screen.getByLabelText(/physical output/i)).toHaveValue('speakers-id');
expect(screen.getByText(/VB-CABLE/i)).toBeInTheDocument();
```

Mock callbacks and verify they are called exactly once.

- [ ] **Step 3: Run the focused UI test and confirm failure**

```powershell
npm run test --workspace @voxveil/ui -- SystemAudioEndpoints.test.tsx
```

Expected: FAIL until new props/actions exist.

- [ ] **Step 4: Add Tauri client methods**

```ts
listAudioOutputs: () => call<AudioOutput[]>('list_audio_outputs'),
setPhysicalAudioOutput: (endpointId: string) => call<void>('set_physical_audio_output', { endpointId }),
openWindowsSoundSettings: () => call<void>('open_windows_sound_settings'),
openVbCableDownload: () => call<void>('open_vb_cable_download'),
```

- [ ] **Step 5: Load outputs and expose actions from `useVoxveilState`**

Maintain `physicalOutputs: AudioOutput[]`; refresh it with native state. Selection calls `setPhysicalAudioOutput`, then refreshes native state. The fixed link/settings actions call their matching commands and surface errors through the existing system-audio error field.

- [ ] **Step 6: Make the system-audio panel visible for routing-required state**

Change `HomeScreen.tsx` to:

```ts
const showSystemAudio =
  state.backendStatus === 'component-required'
  || state.backendStatus === 'routing-required'
  || model.systemAudioEndpoints.length > 0;
```

- [ ] **Step 7: Add strategy-aware panel controls**

In `SystemAudioEndpoints.tsx`:

- missing VB-CABLE + no loaded APO: show `Get VB-CABLE`;
- `routing-required`: show `Open Sound settings` and the physical-output selector;
- `vb-cable-relay`: show backend badge `VB-CABLE` and selected physical destination;
- existing `installable` APO endpoint controls remain available and unchanged.

Use a `<select aria-label={t('systemAudio.physicalOutput')}>` whose values are endpoint IDs, not names.

- [ ] **Step 8: Add translations in all seven locale files**

Add the same keys in every `system-audio.json`:

```json
{
  "getVbCable": "Get VB-CABLE",
  "vbCableRequired": "Install the standard VB-CABLE virtual audio device to enable system-wide processing.",
  "openSoundSettings": "Open Sound settings",
  "routeToCable": "Set CABLE Input as the Windows output, then return to Voxveil.",
  "physicalOutput": "Physical output",
  "backendVbCable": "VB-CABLE",
  "backendApo": "Voxveil APO"
}
```

Translate values naturally for non-English locale files; keep product names `VB-CABLE`, `CABLE Input`, and `Voxveil` unchanged.

- [ ] **Step 9: Run UI tests, typecheck, and i18n quality**

```powershell
npm run test --workspace @voxveil/ui -- SystemAudioEndpoints.test.tsx
npm run typecheck
npm run quality:i18n
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add ui/lib/types.ts ui/lib/tauri.ts ui/app/useVoxveilState.ts ui/features/home/HomeScreen.tsx ui/features/home/SystemAudioEndpoints.tsx ui/features/home/SystemAudioEndpoints.test.tsx locales/*/system-audio.json
git commit -m "feat(windows): add VB-CABLE onboarding UI"
```

### Task 10: Document Tier 1 accurately and remove stale SysVAD production claims

**Files:**
- Modify: `README.md`
- Modify: `docs/specs/platform/windows.md`
- Modify: `docs/specs/platform/windows-dev-relay.md`

**Interfaces:**
- Documentation must match runtime precedence and explicitly distinguish third-party dependency from bundled components.

- [ ] **Step 1: Update README Windows architecture**

Document exactly:

```text
Loaded Voxveil APO -> native in-process processing.
Otherwise: standard VB-CABLE CABLE Input -> WASAPI loopback -> Voxveil DSP -> selected physical output.
Future: Microsoft-signed Voxveil virtual endpoint reuses the same relay.
```

State that VB-CABLE is downloaded from VB-Audio by the user and is not redistributed by Voxveil.

- [ ] **Step 2: Correct `windows.md` production status**

Remove any statement that the old test-signed SysVAD relay is the current shipping production path. Describe `component-required`, `routing-required`, `faulted`, and `ready` semantics.

- [ ] **Step 3: Mark `windows-dev-relay.md` as historical/development-only**

Keep the old SysVAD material only as historical context and point to the signed-audio-paths design and Tier 2 plan.

- [ ] **Step 4: Run repository quality checks**

```powershell
npm run quality
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/specs/platform/windows.md docs/specs/platform/windows-dev-relay.md
git commit -m "docs(windows): document VB-CABLE production relay"
```

### Task 11: Perform Tier 1 Windows integration verification

**Files:**
- Create: `docs/testing/windows-vb-cable-relay.md`
- Modify only if failures require fixes: files from Tasks 1-10.

**Interfaces:**
- Produces a reproducible manual verification matrix; this is required because the GitHub connector cannot exercise real Windows audio hardware.

- [ ] **Step 1: Write the verification matrix**

Include checkboxes for:

```text
VB-CABLE absent -> component-required, Get VB-CABLE visible.
VB-CABLE installed but CABLE Input not default -> routing-required.
CABLE Input default + safe physical output selected -> enable starts relay and state becomes ready.
Centered stereo signal changes audibly at vocal=0 and passes unchanged at vocal=100.
Physical endpoint removal -> relay faults and master enable clears.
Selecting cable as physical output is rejected.
App restart restores the physical endpoint ID when still present.
Stale physical endpoint ID falls back to a safe non-virtual render endpoint.
Loaded APO wins and no VB-CABLE relay worker starts.
Disable/app exit stops capture/render cleanly.
Windows 10 supported desktop and current Windows 11 production release both tested.
USB, HDMI/DisplayPort, Bluetooth, and onboard speakers/headphones exercised where available.
```

- [ ] **Step 2: Run automated verification before hardware testing**

```powershell
cargo test --workspace
npm test
npm run typecheck
npm run quality
npm run build:windows
```

Expected: all commands PASS on a configured Windows development machine with MSBuild/WDK prerequisites.

- [ ] **Step 3: Run the hardware matrix**

Follow `docs/testing/windows-vb-cable-relay.md` and record date, Windows build number, VB-CABLE package version, endpoint names, and pass/fail for each row. Do not record captured audio.

- [ ] **Step 4: Commit the verification document and any proven fixes**

```bash
git add docs/testing/windows-vb-cable-relay.md
git commit -m "test(windows): document VB-CABLE relay verification"
```

Tier 1 is complete only when automated checks pass and the real-device matrix confirms that unprocessed audio does not simultaneously leak directly to the physical output.