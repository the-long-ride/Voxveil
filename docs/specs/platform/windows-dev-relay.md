# Windows Development Audio Relay

> Historical/development context. This document describes the earlier SysVAD-based relay experiment and is not the current end-user interception path. For the current architecture see `docs/superpowers/specs/2026-09-14-windows-signed-audio-paths-design.md`, `docs/specs/platform/windows.md`, and the Tier 2 signed-driver plan.

## Historical purpose

The original Windows all-output experiment established the relay shape:

```text
Windows applications
  -> controlled virtual render endpoint
  -> WASAPI loopback capture
  -> voxveil-windows-audio
  -> Classic DSP / optional AI
  -> physical output
```

The separate virtual capture/monitor endpoint was not required because WASAPI loopback can capture the mix from a render endpoint.

## Superseded development path

Earlier revisions used a compatible Microsoft SysVAD sample endpoint and test-signed development packaging. They also referenced a GitHub Actions portable bundle. That workflow no longer exists and must not be restored as part of the current Windows audio work.

Test-signed SysVAD output is development-only and cannot be treated as a normal Secure Boot/end-user package.

## Current relay routes

The current relay prefers the verified Voxveil virtual render endpoint, `Voxveil Input`. If it is unavailable, the driver-free endpoint relay captures the default physical render endpoint and sends processed audio to a different, explicitly selected physical output.

Both routes share the same [buffering policy](windows.md#relay-buffering): request a 100 ms render buffer and prime it with silence, keep a 64 ms software reserve, fill measured capture holes with silence, correct clock drift, and drop the oldest frames at the FIFO limit.

Current readiness semantics are:

- no loaded APO, Voxveil virtual endpoint, or default physical source: `component-required`;
- source not default, or no distinct safe physical sink: `routing-required`;
- relay startup/runtime failure: `faulted`;
- capture + processing + physical render active: `ready`.

## First-party driver

Tier 2 uses SysVAD only as source material for a Voxveil-owned render-only virtual driver with stable Voxveil identities, reproducible package/CAB tooling, and explicit Microsoft signing/qualification gates. That first-party endpoint reuses the same user-mode relay instead of creating a second processing implementation.

See `docs/superpowers/plans/2026-09-14-windows-tier2-signed-virtual-driver.md` for the implementation plan and `docs/superpowers/plans/2026-09-14-windows-signed-audio-paths-review-notes.md` for authoritative signing corrections.
