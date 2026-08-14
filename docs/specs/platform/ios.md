# iOS Audio Platform Specification

## Standard edition

Standard iOS must stay within public platform APIs and make capture limitations visible. It must not claim universal interception of other applications when iOS does not grant that capability.

## Pro System edition

True system-wide processing is a privileged/jailbreak-oriented distribution path separate from ordinary App Store-style installation. Privileged components communicate through a narrow local protocol and must be removable without leaving routing changes behind.

## Safety

- Standard and Pro artifacts are distinct;
- no private API in the Standard binary;
- no runtime component download;
- local-only processing;
- calls/communication audio bypass by default;
- fail to clean bypass rather than silence the device if the privileged component disappears.

## Verification

Test foreground/background transitions, route changes, Bluetooth/AirPlay where applicable, interruption handling, privileged-component loss, and clean uninstall/restore behavior.

## Foundation status

Capability metadata and shared interfaces exist. Native Standard and privileged Pro routing implementations are later milestones.
