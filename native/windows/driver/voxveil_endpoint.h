#pragma once

// Portions of this endpoint table are derived from Microsoft Windows Driver Samples SysVAD.
// Copyright (c) Microsoft Corporation. Licensed under the Microsoft Public License (MS-PL).
// Voxveil-specific identities and render-only reductions are documented in NOTICE.md.

#include <sysvad.h>
#include "simple.h"
#include "voxveil_topology.h"
#include "voxveil_wavert.h"

NTSTATUS
CreateMiniportWaveRTSYSVAD(
    _Out_ PUNKNOWN*,
    _In_ REFCLSID,
    _In_opt_ PUNKNOWN,
    _In_ POOL_FLAGS,
    _In_ PUNKNOWN,
    _In_opt_ PVOID,
    _In_ PENDPOINT_MINIPAIR);

NTSTATUS
CreateMiniportTopologySYSVAD(
    _Out_ PUNKNOWN*,
    _In_ REFCLSID,
    _In_opt_ PUNKNOWN,
    _In_ POOL_FLAGS,
    _In_ PUNKNOWN,
    _In_opt_ PVOID,
    _In_ PENDPOINT_MINIPAIR);

static PHYSICALCONNECTIONTABLE VoxveilTopologyPhysicalConnections[] = {
    {
        KSPIN_TOPO_WAVEOUT_SOURCE,
        KSPIN_WAVE_RENDER3_SOURCE,
        CONNECTIONTYPE_WAVE_OUTPUT,
    },
};

static ENDPOINT_MINIPAIR VoxveilMiniports = {
    eSpeakerDevice,
    L"TopologyVoxveil",
    nullptr,
    CreateMiniportTopologySYSVAD,
    &VoxveilTopoMiniportFilterDescriptor,
    0,
    nullptr,
    L"WaveVoxveil",
    nullptr,
    CreateMiniportWaveRTSYSVAD,
    &VoxveilWaveMiniportFilterDescriptor,
    0,
    nullptr,
    VOXVEIL_DEVICE_MAX_CHANNELS,
    VoxveilPinDeviceFormatsAndModes,
    SIZEOF_ARRAY(VoxveilPinDeviceFormatsAndModes),
    VoxveilTopologyPhysicalConnections,
    SIZEOF_ARRAY(VoxveilTopologyPhysicalConnections),
    ENDPOINT_NO_FLAGS,
    nullptr,
    0,
    nullptr,
};

static PENDPOINT_MINIPAIR g_RenderEndpoints[] = { &VoxveilMiniports };
#define g_cRenderEndpoints (SIZEOF_ARRAY(g_RenderEndpoints))

// Adapter code expects the symbol even when the count is zero.
static PENDPOINT_MINIPAIR g_CaptureEndpoints[] = { nullptr };
#define g_cCaptureEndpoints 0

static_assert(g_cRenderEndpoints == 1, "Voxveil production driver exposes exactly one render endpoint");
static_assert(g_cCaptureEndpoints == 0, "Voxveil production driver must not expose capture endpoints");

#define g_MaxMiniports ((g_cRenderEndpoints + g_cCaptureEndpoints) * 2)
