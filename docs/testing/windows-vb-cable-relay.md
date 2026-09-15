# Windows VB-CABLE Relay Verification

Date created: 2026-09-14

This matrix verifies the Tier 1 Windows relay on real Windows hardware. Do not record or commit captured audio. Record only configuration metadata and pass/fail observations.

## Environment

- Windows edition/build: ______________________________
- Voxveil commit: _____________________________________
- VB-CABLE package/version: ___________________________
- Primary physical endpoint(s): _______________________
- Audio adapter/driver versions: ______________________
- Test operator/date: _________________________________

## Automated prerequisites

Run from an x64 Developer PowerShell with the required Windows SDK/WDK/MSBuild environment:

```powershell
cargo test --workspace
npm test
npm run typecheck
npm run quality
npm run build:windows
```

Record results:

- [ ] `cargo test --workspace` passes.
- [ ] `npm test` passes.
- [ ] `npm run typecheck` passes.
- [ ] `npm run quality` passes.
- [ ] `npm run build:windows` passes.

## Runtime matrix

### Dependency and routing states

- [ ] With VB-CABLE absent and no loaded Voxveil APO, backend status is `component-required`.
- [ ] In that state, the UI shows **Get VB-CABLE**.
- [ ] The button opens only the official VB-Audio VB-CABLE page.
- [ ] After standard VB-CABLE installation, with `CABLE Input` not the ordinary Windows default render endpoint, backend status is `routing-required`.
- [ ] **Open Sound settings** opens Windows Sound settings.
- [ ] After making `CABLE Input` the ordinary Windows default render endpoint and selecting a safe physical output, enabling processing starts the relay and backend status becomes `ready`.

### DSP behavior

Use non-sensitive test audio only.

- [ ] At vocal level `100`, the processed relay is perceptually equivalent to bypass for the supported stereo float path.
- [ ] At vocal level `0`, centered stereo content changes audibly according to the Classic DSP behavior.
- [ ] Live vocal-level changes update the active relay without restarting the application.
- [ ] No duplicate unprocessed stream is audible directly from the selected physical output while the relay route is active.

### Safety and recovery

- [ ] Attempting to select the interception endpoint as the physical output is rejected.
- [ ] A known virtual endpoint is not offered as a physical sink.
- [ ] Removing the selected physical endpoint causes the relay to stop/fault rather than recurse or silently claim readiness.
- [ ] After an unrecoverable relay error, master processing no longer claims an active `ready` backend.
- [ ] Disabling processing stops capture/render cleanly.
- [ ] Closing Voxveil stops the relay cleanly.
- [ ] Restarting Voxveil restores the previously selected physical endpoint ID when that endpoint still exists.
- [ ] A stale saved physical endpoint ID falls back to a safe non-virtual render endpoint or reports `routing-required` if none exists.

### Backend precedence

- [ ] With a Voxveil APO actually loaded by AudioDG, backend kind is `apo`.
- [ ] A loaded APO wins over an installed/default VB-CABLE route.
- [ ] No VB-CABLE relay worker is started while the loaded APO owns processing.

### Endpoint coverage

Exercise available endpoint classes and note any unsupported combinations:

| Endpoint class | Device | Windows build | Result | Notes |
| --- | --- | --- | --- | --- |
| Onboard speakers/headphones | | | ☐ Pass ☐ Fail ☐ N/A | |
| USB audio | | | ☐ Pass ☐ Fail ☐ N/A | |
| HDMI / DisplayPort | | | ☐ Pass ☐ Fail ☐ N/A | |
| Bluetooth | | | ☐ Pass ☐ Fail ☐ N/A | |

### OS coverage

- [ ] Supported Windows 10 desktop release tested, if included in the current Voxveil support policy.
- [ ] Current supported Windows 11 production release tested.

## Sample-rate / format coverage

For each supported physical endpoint, repeat with at least two shared-mode sample-rate configurations where the hardware/driver permits it.

| Source shared format | Physical shared format | Result | Notes |
| --- | --- | --- | --- |
| 2ch float / 44.1 kHz | | ☐ Pass ☐ Fail ☐ N/A | |
| 2ch float / 48 kHz | | ☐ Pass ☐ Fail ☐ N/A | |
| Other supported rate | | ☐ Pass ☐ Fail ☐ N/A | |

The initial relay intentionally requires the interception source format to be stereo 32-bit float. Unsupported source formats must fail closed rather than report `ready`.

## Release decision

Tier 1 is release-ready only after:

- all automated prerequisites pass on a configured Windows development machine;
- the relevant OS/device matrix passes;
- no unprocessed duplicate stream reaches the physical destination;
- failure cases clear readiness correctly;
- the final test record identifies the exact Voxveil commit and Windows builds tested.

Result: ☐ Pass ☐ Fail ☐ Blocked

Notes:

________________________________________________________________________

________________________________________________________________________
