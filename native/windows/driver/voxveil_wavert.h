#pragma once

// Portions of this WaveRT table are derived from Microsoft Windows Driver Samples SysVAD.
// Copyright (c) Microsoft Corporation. Licensed under the Microsoft Public License (MS-PL).
// Voxveil-specific float32/render-only reductions are documented in NOTICE.md.

#include <sysvad.h>
#include "simple.h"

#define VOXVEIL_DEVICE_MAX_CHANNELS 2
#define VOXVEIL_MAX_SYSTEM_STREAMS 6

static KSDATAFORMAT_WAVEFORMATEXTENSIBLE VoxveilHostFormats[] = {
    {
        {
            sizeof(KSDATAFORMAT_WAVEFORMATEXTENSIBLE),
            0,
            0,
            0,
            STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
            STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT),
            STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX),
        },
        {
            {
                WAVE_FORMAT_EXTENSIBLE,
                2,
                48000,
                384000,
                8,
                32,
                sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX),
            },
            32,
            KSAUDIO_SPEAKER_STEREO,
            STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT),
        },
    },
};

static MODE_AND_DEFAULT_FORMAT VoxveilHostModes[] = {
    { STATIC_AUDIO_SIGNALPROCESSINGMODE_RAW, &VoxveilHostFormats[0].DataFormat },
    { STATIC_AUDIO_SIGNALPROCESSINGMODE_DEFAULT, &VoxveilHostFormats[0].DataFormat },
    { STATIC_AUDIO_SIGNALPROCESSINGMODE_MEDIA, &VoxveilHostFormats[0].DataFormat },
    { STATIC_AUDIO_SIGNALPROCESSINGMODE_MOVIE, &VoxveilHostFormats[0].DataFormat },
};

// No hardware loopback pin is exposed. WASAPI software loopback captures the
// shared-mode render mix from the audio engine on this endpoint.
static PIN_DEVICE_FORMATS_AND_MODES VoxveilPinDeviceFormatsAndModes[] = {
    {
        SystemRenderPin,
        VoxveilHostFormats,
        SIZEOF_ARRAY(VoxveilHostFormats),
        VoxveilHostModes,
        SIZEOF_ARRAY(VoxveilHostModes),
    },
    {
        BridgePin,
        nullptr,
        0,
        nullptr,
        0,
    },
};

static KSDATARANGE_AUDIO VoxveilHostRange = {
    {
        sizeof(KSDATARANGE_AUDIO),
        KSDATARANGE_ATTRIBUTES,
        0,
        0,
        STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
        STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT),
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX),
    },
    2,
    32,
    32,
    48000,
    48000,
};

static PKSDATARANGE VoxveilHostRangePointers[] = {
    PKSDATARANGE(&VoxveilHostRange),
    PKSDATARANGE(&PinDataRangeAttributeList),
};

static KSDATARANGE VoxveilBridgeRange = {
    sizeof(KSDATARANGE),
    0,
    0,
    0,
    STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
    STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
    STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE),
};

static PKSDATARANGE VoxveilBridgeRangePointers[] = {
    &VoxveilBridgeRange,
};

static PCPIN_DESCRIPTOR VoxveilWavePins[] = {
    {
        VOXVEIL_MAX_SYSTEM_STREAMS,
        VOXVEIL_MAX_SYSTEM_STREAMS,
        0,
        nullptr,
        {
            0, nullptr,
            0, nullptr,
            SIZEOF_ARRAY(VoxveilHostRangePointers),
            VoxveilHostRangePointers,
            KSPIN_DATAFLOW_IN,
            KSPIN_COMMUNICATION_SINK,
            &KSCATEGORY_AUDIO,
            nullptr,
            0,
        },
    },
    {
        0,
        0,
        0,
        nullptr,
        {
            0, nullptr,
            0, nullptr,
            SIZEOF_ARRAY(VoxveilBridgeRangePointers),
            VoxveilBridgeRangePointers,
            KSPIN_DATAFLOW_OUT,
            KSPIN_COMMUNICATION_NONE,
            &KSCATEGORY_AUDIO,
            nullptr,
            0,
        },
    },
};

static PCCONNECTION_DESCRIPTOR VoxveilWaveConnections[] = {
    { PCFILTER_NODE, KSPIN_WAVE_RENDER3_SINK_SYSTEM, PCFILTER_NODE, KSPIN_WAVE_RENDER3_SOURCE },
};

static PCPROPERTY_ITEM VoxveilWaveProperties[] = {
    {
        &KSPROPSETID_Pin,
        KSPROPERTY_PIN_PROPOSEDATAFORMAT,
        KSPROPERTY_TYPE_SET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_WaveFilter,
    },
    {
        &KSPROPSETID_Pin,
        KSPROPERTY_PIN_PROPOSEDATAFORMAT2,
        KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_WaveFilter,
    },
};

DEFINE_PCAUTOMATION_TABLE_PROP(AutomationVoxveilWaveFilter, VoxveilWaveProperties);

static PCFILTER_DESCRIPTOR VoxveilWaveMiniportFilterDescriptor = {
    0,
    &AutomationVoxveilWaveFilter,
    sizeof(PCPIN_DESCRIPTOR),
    SIZEOF_ARRAY(VoxveilWavePins),
    VoxveilWavePins,
    sizeof(PCNODE_DESCRIPTOR),
    0,
    nullptr,
    SIZEOF_ARRAY(VoxveilWaveConnections),
    VoxveilWaveConnections,
    0,
    nullptr,
};
