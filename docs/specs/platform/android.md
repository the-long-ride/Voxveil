# Android Audio Platform Specification

## Standard edition

Use only public Android capture/playback APIs and respect applications that disallow playback capture. Standard edition must report capability limits rather than imply unrestricted device-wide interception.

## Pro System edition

Pro System targets true system-wide routing through an explicitly privileged/root installation path. Privileged components must be versioned, locally packaged, integrity-checked, reversible, and isolated from UI/business logic.

## Controls

Expose the same master, per-app, vocal level, quality, engine, and routing concepts as desktop. Mobile layouts use the responsive Voxveil shell and platform quick controls where available.

## Safety

- no root command execution from arbitrary UI text;
- narrowly defined privileged protocol;
- no runtime binary download;
- no telemetry or cloud inference;
- battery/thermal pressure may reduce processing quality but must not silently exceed the user's quality ceiling;
- calls remain bypassed by default.

## Verification

Test public capture restrictions, audio-focus changes, Bluetooth, headset changes, background lifecycle, thermal degradation, root module version mismatch, and safe bypass if privileged routing fails.

## Foundation status

Standard/Pro capability metadata and shared interfaces exist. Android native/privileged routing modules remain a later milestone.
