# Linux Audio Platform Specification

## Scope

PipeWire is the primary Linux routing target. The adapter owns stream discovery, graph routing, virtual sinks/sources, and restoration while the shared Rust core owns processing.

## Standard edition

Use the user PipeWire graph to discover playback streams and route all or selected streams through Voxveil. Prefer supported PipeWire APIs and portals over shelling out to external commands.

## Pro System edition

Pro System is normally unnecessary on modern PipeWire desktops. It exists for feature parity and future system-level deployment scenarios, not as an excuse to require root for ordinary Linux use.

## Safety

- do not modify persistent PipeWire configuration unless the user explicitly requests it;
- restore graph links on shutdown/failure;
- isolate unexpected sample formats through the conversion boundary;
- communication streams remain bypassed by default;
- no network dependency in the runtime path.

## Verification

Test stream appearance/disappearance, graph restoration, default sink changes, virtual sink behavior, Bluetooth reconnect, and underrun recovery on at least one PipeWire-first distribution.

## Foundation status

Capability contracts exist. Native PipeWire capture/routing code is a later milestone.
