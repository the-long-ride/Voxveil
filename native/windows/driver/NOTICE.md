# Voxveil Virtual Audio driver source notice

The Voxveil virtual-audio kernel driver is derived from the Microsoft Windows Driver Samples SysVAD architecture at the pinned upstream revision:

`microsoft/Windows-driver-samples@67d81f217bc01edf7a4320e4911c11065635acfa`

The upstream source is licensed under the Microsoft Public License (MS-PL). The repository preserves the license and provenance under:

- `third_party/microsoft/windows-driver-samples/LICENSE.txt`
- `third_party/microsoft/windows-driver-samples/SOURCE_REVISION`
- `third_party/microsoft/windows-driver-samples/README.voxveil.md`

## Voxveil-owned changes

Voxveil intentionally narrows the sample architecture to one root-enumerated render endpoint:

- hardware ID: `Root\VoxveilVirtualAudio`
- device: `Voxveil Virtual Audio`
- render endpoint: `Voxveil Input`
- host format: stereo IEEE float32, 48 kHz
- no microphone/capture endpoint
- no Bluetooth/USB/HDMI/SPDIF sideband endpoint
- no hardware offload pin
- no hardware loopback pin; the application uses standard WASAPI software loopback on the render endpoint
- no sample APOs
- no keyword detector
- no DevCon dependency
- no production/test certificate material in source control

The generic SysVAD WaveRT/topology/adapter machinery remains derived from the pinned Microsoft sample and is compiled from the imported `third_party` snapshot. Voxveil-owned endpoint, topology, and WaveRT tables retain an in-file upstream provenance/license marker in addition to this notice.

## Audio persistence removal

Microsoft's SysVAD sample contains `CSaveData`, which can write sample render data to disk for development diagnostics. Voxveil does not compile the upstream `savedata.cpp` implementation. Instead, `voxveil_savedata.cpp` implements the same class contract as a permanent no-op. This is defense in depth for Voxveil's no-audio-persistence requirement; changing sample registry settings cannot enable file output.

## Sample tone generation removal

The generic SysVAD WaveRT stream references the sample `ToneGenerator` class. Voxveil does not compile the upstream `tonegenerator.cpp` implementation. `voxveil_tonegenerator.cpp` satisfies the class contract but emits silence only, so the production driver cannot synthesize the SysVAD sample tone even if generic stream code initializes that helper.

## Signing boundary

`VoxveilVirtualAudio.vcxproj` has WDK signing mode disabled. Local compilation produces unsigned development output only. Repository scripts generate and validate submission material, but EV/private-key and Microsoft Partner Center operations occur outside the repository. See `docs/release/windows-driver-signing.md`.
