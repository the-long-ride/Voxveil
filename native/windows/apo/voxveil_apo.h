#pragma once

#include "control_state.h"

#include <atlbase.h>
#include <atlcom.h>
#include <audioenginebaseapo.h>
#include <baseaudioprocessingobject.h>

#include <cstdint>

extern const CLSID CLSID_VoxveilApo;

class ATL_NO_VTABLE CVoxveilApo :
    public CComObjectRootEx<CComMultiThreadModel>,
    public CComCoClass<CVoxveilApo, &CLSID_VoxveilApo>,
    public CBaseAudioProcessingObject,
    public IAudioSystemEffects {
public:
    CVoxveilApo();
    ~CVoxveilApo() override = default;

    DECLARE_NO_REGISTRY()

    BEGIN_COM_MAP(CVoxveilApo)
        COM_INTERFACE_ENTRY(IAudioProcessingObject)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectRT)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectConfiguration)
        COM_INTERFACE_ENTRY(IAudioSystemEffects)
    END_COM_MAP()

    DECLARE_PROTECT_FINAL_CONSTRUCT()

    STDMETHOD(Initialize)(UINT32 cb_data_size, BYTE* data) override;
    STDMETHOD_(void, APOProcess)(
        UINT32 input_count,
        APO_CONNECTION_PROPERTY** inputs,
        UINT32 output_count,
        APO_CONNECTION_PROPERTY** outputs) override;
    STDMETHOD(GetLatency)(HNSTIME* latency) override;
    STDMETHOD(LockForProcess)(
        UINT32 input_count,
        APO_CONNECTION_DESCRIPTOR** inputs,
        UINT32 output_count,
        APO_CONNECTION_DESCRIPTOR** outputs) override;

    static const CRegAPOProperties<1> registration_properties;

private:
    voxveil::ControlState control_;
    std::uint32_t channels_{2};
};

OBJECT_ENTRY_AUTO(CLSID_VoxveilApo, CVoxveilApo)
