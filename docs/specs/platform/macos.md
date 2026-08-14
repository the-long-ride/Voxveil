# macOS Audio Platform Specification

## Scope

The macOS adapter uses supported Core Audio capture/routing mechanisms and, when required, a signed system/driver component for a virtual processed endpoint.

## Standard edition

The ordinary application should use public macOS audio APIs for capturable system/process audio. Per-app identity and permissions must be represented clearly when the OS limits access.

## Pro System edition

Where insertion or a virtual endpoint requires a privileged system component, Pro System packages it separately with explicit installation, signing, and removal behavior.

## Safety

- public APIs for Standard edition;
- no hidden persistence mechanisms;
- no runtime code download;
- microphone input is outside the v1 processing path;
- calls/VoIP bypass by default;
- audio restoration must survive device changes and application crashes.

## Verification

Cover default-output changes, aggregate/external interfaces, sleep/wake, Bluetooth, per-process lifecycle, virtual endpoint fan-out, and permission denial.

## Foundation status

Shared contracts and macOS capability metadata exist. Native Core Audio/system-extension implementation is not yet implemented.
