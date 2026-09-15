#pragma once

#include <atlbase.h>
#include <atlcom.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <baseaudioprocessingobject.h>
#include <mmdeviceapi.h>
#include <propsys.h>
#include <servprov.h>

#include "VoxveilApoIds.h"
#include "VoxveilApoPolicy.h"
#include "VoxveilSharedState.h"

// {F3F2A99F-8FB7-4B88-949E-448BF8A05221}
inline const GUID CLSID_VoxveilApo =
{ 0xf3f2a99f, 0x8fb7, 0x4b88, { 0x94, 0x9e, 0x44, 0x8b, 0xf8, 0xa0, 0x52, 0x21 } };

class ATL_NO_VTABLE CVoxveilApo :
    public CComObjectRootEx<CComMultiThreadModel>,
    public CComCoClass<CVoxveilApo, &CLSID_VoxveilApo>,
    public CBaseAudioProcessingObject,
    public IAudioSystemEffects3
{
public:
    CVoxveilApo() noexcept;
    ~CVoxveilApo() noexcept;

    DECLARE_NO_REGISTRY()
    DECLARE_NOT_AGGREGATABLE(CVoxveilApo)
    DECLARE_PROTECT_FINAL_CONSTRUCT()

    BEGIN_COM_MAP(CVoxveilApo)
        COM_INTERFACE_ENTRY(IAudioSystemEffects)
        COM_INTERFACE_ENTRY(IAudioSystemEffects2)
        COM_INTERFACE_ENTRY(IAudioSystemEffects3)
        COM_INTERFACE_ENTRY(IAudioProcessingObject)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectRT)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectConfiguration)
    END_COM_MAP()

    STDMETHOD(Initialize)(UINT32 cbDataSize, BYTE* data) override;
    STDMETHOD(GetLatency)(HNSTIME* time) override;
    STDMETHOD(GetEffectsList)(LPGUID* effects, UINT* count, HANDLE event) override;
    STDMETHOD(GetControllableSystemEffectsList)(
        AUDIO_SYSTEMEFFECT** effects,
        UINT* count,
        HANDLE event) override;
    STDMETHOD(SetAudioSystemEffectState)(
        GUID effectId,
        AUDIO_SYSTEMEFFECT_STATE state) override;
    STDMETHOD_(void, APOProcess)(
        UINT32 inputCount,
        APO_CONNECTION_PROPERTY** inputs,
        UINT32 outputCount,
        APO_CONNECTION_PROPERTY** outputs) override;

    static const CRegAPOProperties<1> sm_RegProperties;

private:
    HRESULT ReplaceEffectsChangedEvent(HANDLE event) noexcept;
    HRESULT InitializeCapx(const APOInitSystemEffects3* init3) noexcept;
    AUDIO_SYSTEMEFFECT_STATE CurrentSystemEffectState() const noexcept;
    void SignalEffectsChanged() noexcept;

    HANDLE mapping_ = nullptr;
    voxveil::SharedState* state_ = nullptr;
    bool countedLoadedInstance_ = false;
    bool countedCapxInstance_ = false;
    bool initializeForDiscoveryOnly_ = false;
    GUID audioProcessingMode_ = AUDIO_SIGNALPROCESSINGMODE_DEFAULT;

    SRWLOCK effectsLock_ = SRWLOCK_INIT;
    HANDLE effectsChangedEvent_ = nullptr;
    CComPtr<IPropertyStore> defaultStore_;
    CComPtr<IPropertyStore> userStore_;
    CComPtr<IPropertyStore> volatileStore_;
    CComPtr<IAudioProcessingObjectLoggingService> loggingService_;
};

OBJECT_ENTRY_AUTO(CLSID_VoxveilApo, CVoxveilApo)
