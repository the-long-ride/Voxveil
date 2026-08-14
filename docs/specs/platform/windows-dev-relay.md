# Windows Development Audio Relay

## Purpose

Windows all-output processing uses a virtual render endpoint plus the user-mode Rust relay:

```text
Windows apps
  -> virtual render endpoint (Voxveil Output / compatible SysVAD endpoint)
  -> WASAPI loopback capture on that render endpoint
  -> voxveil-windows-audio
  -> Classic DSP / optional AI
  -> physical output
```

The separate `Voxveil Monitor` capture endpoint is no longer required. WASAPI loopback captures the system mix directly from the virtual render endpoint.

## GitHub Actions development component

`.github/workflows/windows-portable.yml` builds the app together with Microsoft's SysVAD sample and the DevCon helper. The uploaded artifact contains `voxveil.exe` plus `system-audio/driver`, `system-audio/devcon.exe`, and install/uninstall scripts.

This is a development path only. The SysVAD package produced by CI is test-signed. Windows does not load test-signed kernel drivers in normal production mode; a test machine must be prepared for TESTSIGNING, while a normal end-user build needs a Microsoft production-signed Voxveil driver package.

## Runtime contract

- no virtual render endpoint: `backendStatus = component-required`;
- virtual endpoint installed but not default: `backendStatus = routing-required`;
- virtual endpoint is default and the relay starts: `backendStatus = ready`;
- communication bypass and per-app routing remain later milestones.

## Production boundary

The development artifact proves the full app -> virtual endpoint -> WASAPI loopback -> DSP -> physical output path. It is not the production distribution architecture. Production still requires a reviewed Voxveil driver identity, stable hardware IDs, HLK/Partner Center signing as applicable, uninstall/upgrade handling, and crash-safe routing restoration.
