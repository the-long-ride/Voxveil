#pragma once

// Portions of this topology table are derived from Microsoft Windows Driver Samples SysVAD.
// Copyright (c) Microsoft Corporation. Licensed under the Microsoft Public License (MS-PL).
// Voxveil-specific identities and render-only reductions are documented in NOTICE.md.

#include <sysvad.h>
#include "simple.h"
#include "voxveil_ids.h"

static KSDATARANGE VoxveilTopoBridgeRange = {
    sizeof(KSDATARANGE),
    0,
    0,
    0,
    STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO),
    STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
    STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE),
};

static PKSDATARANGE VoxveilTopoBridgeRangePointers[] = {
    &VoxveilTopoBridgeRange,
};

static PCPIN_DESCRIPTOR VoxveilTopoPins[] = {
    {
        0, 0, 0, nullptr,
        {
            0, nullptr,
            0, nullptr,
            SIZEOF_ARRAY(VoxveilTopoBridgeRangePointers),
            VoxveilTopoBridgeRangePointers,
            KSPIN_DATAFLOW_IN,
            KSPIN_COMMUNICATION_NONE,
            &KSCATEGORY_AUDIO,
            nullptr,
            0,
        },
    },
    {
        0, 0, 0, nullptr,
        {
            0, nullptr,
            0, nullptr,
            SIZEOF_ARRAY(VoxveilTopoBridgeRangePointers),
            VoxveilTopoBridgeRangePointers,
            KSPIN_DATAFLOW_OUT,
            KSPIN_COMMUNICATION_NONE,
            &KSNODETYPE_SPEAKER,
            &KSNODETYPE_VOXVEIL_VIRTUAL_SPEAKER,
            0,
        },
    },
};

static PCCONNECTION_DESCRIPTOR VoxveilTopoConnections[] = {
    { PCFILTER_NODE, KSPIN_TOPO_WAVEOUT_SOURCE, PCFILTER_NODE, KSPIN_TOPO_LINEOUT_DEST },
};

static PCFILTER_DESCRIPTOR VoxveilTopoMiniportFilterDescriptor = {
    0,
    nullptr,
    sizeof(PCPIN_DESCRIPTOR),
    SIZEOF_ARRAY(VoxveilTopoPins),
    VoxveilTopoPins,
    sizeof(PCNODE_DESCRIPTOR),
    0,
    nullptr,
    SIZEOF_ARRAY(VoxveilTopoConnections),
    VoxveilTopoConnections,
    0,
    nullptr,
};
