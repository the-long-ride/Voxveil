#include <initguid.h>
#include "VoxveilApo.h"

#include <algorithm>
#include <cstring>
#include <propvarutil.h>

#pragma warning(disable : 4815)
const AVRT_DATA CRegAPOProperties<1> CVoxveilApo::sm_RegProperties(
    CLSID_VoxveilApo,
    L"Voxveil System Effects APO",
    L"Copyright (c) Voxveil contributors",
    1,
    0,
    __uuidof(IAudioSystemEffects));

CVoxveilApo::CVoxveilApo() noexcept
    : CBaseAudioProcessingObject(sm_RegProperties) {
    state_ = voxveil::OpenOrCreateSharedState(&mapping_);
}

CVoxveilApo::~CVoxveilApo() noexcept {
    AcquireSRWLockExclusive(&effectsLock_);
    if (effectsChangedEvent_ != nullptr) {
        CloseHandle(effectsChangedEvent_);
        effectsChangedEvent_ = nullptr;
    }
    ReleaseSRWLockExclusive(&effectsLock_);

    if (state_ != nullptr) {
        if (countedLoadedInstance_) {
            InterlockedDecrement(&state_->loadedInstances);
        }
        if (countedCapxInstance_) {
            InterlockedDecrement(&state_->capxInstances);
        }
    }
    voxveil::CloseSharedState(mapping_, state_);
    mapping_ = nullptr;
    state_ = nullptr;
}

HRESULT CVoxveilApo::ReplaceEffectsChangedEvent(HANDLE event) noexcept {
    HANDLE duplicate = nullptr;
    if (event != nullptr) {
        if (!DuplicateHandle(
                GetCurrentProcess(),
                event,
                GetCurrentProcess(),
                &duplicate,
                EVENT_MODIFY_STATE,
                FALSE,
                0)) {
            return HRESULT_FROM_WIN32(GetLastError());
        }
    }

    AcquireSRWLockExclusive(&effectsLock_);
    if (effectsChangedEvent_ != nullptr) {
        CloseHandle(effectsChangedEvent_);
    }
    effectsChangedEvent_ = duplicate;
    ReleaseSRWLockExclusive(&effectsLock_);
    return S_OK;
}

void CVoxveilApo::SignalEffectsChanged() noexcept {
    AcquireSRWLockExclusive(&effectsLock_);
    if (effectsChangedEvent_ != nullptr) {
        SetEvent(effectsChangedEvent_);
    }
    ReleaseSRWLockExclusive(&effectsLock_);
}

AUDIO_SYSTEMEFFECT_STATE CVoxveilApo::CurrentSystemEffectState() const noexcept {
    if (state_ == nullptr) {
        return AUDIO_SYSTEMEFFECT_STATE_OFF;
    }
    return InterlockedCompareExchange(&state_->systemEffectEnabled, 0, 0) != 0
        ? AUDIO_SYSTEMEFFECT_STATE_ON
        : AUDIO_SYSTEMEFFECT_STATE_OFF;
}

HRESULT CVoxveilApo::InitializeCapx(const APOInitSystemEffects3* init3) noexcept {
    if (init3 == nullptr || init3->pDeviceCollection == nullptr) {
        return E_INVALIDARG;
    }

    if (init3->pServiceProvider != nullptr) {
        IAudioProcessingObjectLoggingService* logging = nullptr;
        const HRESULT loggingResult = init3->pServiceProvider->QueryService(
            SID_AudioProcessingObjectLoggingService,
            __uuidof(IAudioProcessingObjectLoggingService),
            reinterpret_cast<void**>(&logging));
        if (SUCCEEDED(loggingResult) && logging != nullptr) {
            loggingService_.Attach(logging);
        }
    }

    UINT deviceCount = 0;
    HRESULT hr = init3->pDeviceCollection->GetCount(&deviceCount);
    if (FAILED(hr)) {
        return hr;
    }
    if (deviceCount == 0) {
        return E_UNEXPECTED;
    }

    CComPtr<IMMDevice> endpoint;
    hr = init3->pDeviceCollection->Item(deviceCount - 1, &endpoint);
    if (FAILED(hr)) {
        return hr;
    }

    PROPVARIANT activation{};
    PropVariantInit(&activation);
    hr = InitPropVariantFromCLSID(GUID_VoxveilApoPropertyContext, &activation);
    if (FAILED(hr)) {
        PropVariantClear(&activation);
        return hr;
    }

    IAudioSystemEffectsPropertyStore* rawEffectsStore = nullptr;
    hr = endpoint->Activate(
        __uuidof(IAudioSystemEffectsPropertyStore),
        CLSCTX_ALL,
        &activation,
        reinterpret_cast<void**>(&rawEffectsStore));
    PropVariantClear(&activation);
    if (FAILED(hr)) {
        return hr;
    }

    CComPtr<IAudioSystemEffectsPropertyStore> effectsStore;
    effectsStore.Attach(rawEffectsStore);

    IPropertyStore* rawStore = nullptr;
    hr = effectsStore->OpenDefaultPropertyStore(STGM_READ, &rawStore);
    if (SUCCEEDED(hr)) {
        defaultStore_.Attach(rawStore);
    } else if (hr != E_NOTFOUND) {
        return hr;
    }

    rawStore = nullptr;
    hr = effectsStore->OpenUserPropertyStore(STGM_READWRITE, &rawStore);
    if (SUCCEEDED(hr)) {
        userStore_.Attach(rawStore);
    } else if (hr != E_NOTFOUND) {
        return hr;
    }

    rawStore = nullptr;
    hr = effectsStore->OpenVolatilePropertyStore(STGM_READWRITE, &rawStore);
    if (SUCCEEDED(hr)) {
        volatileStore_.Attach(rawStore);
    } else if (hr != E_NOTFOUND) {
        return hr;
    }

    return S_OK;
}

STDMETHODIMP CVoxveilApo::Initialize(UINT32 cbDataSize, BYTE* data) {
    if (data == nullptr) {
        return E_INVALIDARG;
    }
    if (m_bIsInitialized) {
        return APOERR_ALREADY_INITIALIZED;
    }
    if (state_ == nullptr) {
        return E_UNEXPECTED;
    }

    const voxveil::ApoInitFlavor flavor = voxveil::ClassifyInitSize(cbDataSize);
    if (flavor == voxveil::ApoInitFlavor::Invalid) {
        return E_INVALIDARG;
    }

    audioProcessingMode_ = AUDIO_SIGNALPROCESSINGMODE_DEFAULT;
    initializeForDiscoveryOnly_ = false;

    if (flavor == voxveil::ApoInitFlavor::V3) {
        const auto* init3 = reinterpret_cast<const APOInitSystemEffects3*>(data);
        audioProcessingMode_ = init3->AudioProcessingMode;
        initializeForDiscoveryOnly_ = init3->InitializeForDiscoveryOnly != FALSE;
        const HRESULT hr = InitializeCapx(init3);
        if (FAILED(hr)) {
            return hr;
        }
    } else if (flavor == voxveil::ApoInitFlavor::V2) {
        const auto* init2 = reinterpret_cast<const APOInitSystemEffects2*>(data);
        audioProcessingMode_ = init2->AudioProcessingMode;
    }

    if (voxveil::ShouldCountLoadedInstance(flavor, initializeForDiscoveryOnly_)) {
        InterlockedIncrement(&state_->loadedInstances);
        countedLoadedInstance_ = true;
    }
    if (flavor == voxveil::ApoInitFlavor::V3) {
        InterlockedIncrement(&state_->capxInstances);
        countedCapxInstance_ = true;
    }

    m_bIsInitialized = true;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::GetLatency(HNSTIME* time) {
    if (time == nullptr) {
        return E_POINTER;
    }
    *time = 0;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::GetEffectsList(LPGUID* effects, UINT* count, HANDLE event) {
    if (effects == nullptr || count == nullptr) {
        return E_POINTER;
    }
    *effects = nullptr;
    *count = 0;

    HRESULT hr = ReplaceEffectsChangedEvent(event);
    if (FAILED(hr)) {
        return hr;
    }

    if (IsEqualGUID(audioProcessingMode_, AUDIO_SIGNALPROCESSINGMODE_RAW) ||
        CurrentSystemEffectState() != AUDIO_SYSTEMEFFECT_STATE_ON) {
        return S_OK;
    }

    auto* effectIds = static_cast<LPGUID>(CoTaskMemAlloc(sizeof(GUID)));
    if (effectIds == nullptr) {
        return E_OUTOFMEMORY;
    }
    effectIds[0] = GUID_VoxveilVocalSuppressionEffect;
    *effects = effectIds;
    *count = 1;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::GetControllableSystemEffectsList(
    AUDIO_SYSTEMEFFECT** effects,
    UINT* count,
    HANDLE event) {
    if (effects == nullptr || count == nullptr) {
        return E_POINTER;
    }
    *effects = nullptr;
    *count = 0;

    HRESULT hr = ReplaceEffectsChangedEvent(event);
    if (FAILED(hr)) {
        return hr;
    }

    if (IsEqualGUID(audioProcessingMode_, AUDIO_SIGNALPROCESSINGMODE_RAW)) {
        return S_OK;
    }

    auto* item = static_cast<AUDIO_SYSTEMEFFECT*>(CoTaskMemAlloc(sizeof(AUDIO_SYSTEMEFFECT)));
    if (item == nullptr) {
        return E_OUTOFMEMORY;
    }
    item->id = GUID_VoxveilVocalSuppressionEffect;
    item->canSetState = TRUE;
    item->state = CurrentSystemEffectState();
    *effects = item;
    *count = 1;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::SetAudioSystemEffectState(
    GUID effectId,
    AUDIO_SYSTEMEFFECT_STATE effectState) {
    if (!IsEqualGUID(effectId, GUID_VoxveilVocalSuppressionEffect)) {
        return E_NOTFOUND;
    }
    if (effectState != AUDIO_SYSTEMEFFECT_STATE_OFF &&
        effectState != AUDIO_SYSTEMEFFECT_STATE_ON) {
        return E_INVALIDARG;
    }
    if (state_ == nullptr) {
        return E_UNEXPECTED;
    }

    const LONG enabled = effectState == AUDIO_SYSTEMEFFECT_STATE_ON ? 1 : 0;
    const LONG previous = InterlockedExchange(&state_->systemEffectEnabled, enabled);
    if (previous != enabled) {
        SignalEffectsChanged();
    }
    return S_OK;
}

#pragma AVRT_CODE_BEGIN
STDMETHODIMP_(void) CVoxveilApo::APOProcess(
    UINT32 inputCount,
    APO_CONNECTION_PROPERTY** inputs,
    UINT32 outputCount,
    APO_CONNECTION_PROPERTY** outputs) {
    ATLASSERT(m_bIsLocked);
    if (inputCount == 0 || outputCount == 0 || inputs == nullptr || outputs == nullptr ||
        inputs[0] == nullptr || outputs[0] == nullptr) {
        return;
    }

    auto* input = inputs[0];
    auto* output = outputs[0];
    if (input->u32BufferFlags == BUFFER_INVALID) {
        output->u32ValidFrameCount = 0;
        output->u32BufferFlags = BUFFER_INVALID;
        return;
    }

    const UINT32 frames = input->u32ValidFrameCount;
    const UINT32 channels = GetSamplesPerFrame();
    auto* inputSamples = reinterpret_cast<FLOAT32*>(input->pBuffer);
    auto* outputSamples = reinterpret_cast<FLOAT32*>(output->pBuffer);

    if (inputSamples == nullptr || outputSamples == nullptr || channels == 0) {
        output->u32ValidFrameCount = 0;
        output->u32BufferFlags = BUFFER_INVALID;
        return;
    }

    const size_t sampleCount = static_cast<size_t>(frames) * channels;
    if (input->u32BufferFlags == BUFFER_SILENT) {
        ZeroMemory(outputSamples, sampleCount * sizeof(FLOAT32));
        output->u32BufferFlags = BUFFER_SILENT;
        output->u32ValidFrameCount = frames;
        if (state_ != nullptr && !initializeForDiscoveryOnly_) {
            InterlockedIncrement(&state_->heartbeat);
        }
        return;
    }

    if (outputSamples != inputSamples) {
        CopyMemory(outputSamples, inputSamples, sampleCount * sizeof(FLOAT32));
    }

    if (state_ != nullptr && !initializeForDiscoveryOnly_) {
        InterlockedIncrement(&state_->heartbeat);
        const bool appEnabled = InterlockedCompareExchange(&state_->enabled, 0, 0) != 0;
        const bool systemEffectEnabled =
            InterlockedCompareExchange(&state_->systemEffectEnabled, 0, 0) != 0;
        const LONG vocal = std::clamp<LONG>(
            InterlockedCompareExchange(&state_->vocalPercent, 0, 0), 0, 100);

        if (voxveil::ShouldProcess(appEnabled, systemEffectEnabled, vocal) && channels >= 2) {
            const FLOAT32 centerGain = static_cast<FLOAT32>(vocal) / 100.0f;
            for (UINT32 frame = 0; frame < frames; ++frame) {
                FLOAT32* samples = outputSamples + (static_cast<size_t>(frame) * channels);
                const FLOAT32 left = samples[0];
                const FLOAT32 right = samples[1];
                const FLOAT32 mid = (left + right) * 0.5f;
                const FLOAT32 side = (left - right) * 0.5f;
                const FLOAT32 scaledMid = mid * centerGain;
                samples[0] = scaledMid + side;
                samples[1] = scaledMid - side;
            }
        }
    }

    output->u32BufferFlags = input->u32BufferFlags;
    output->u32ValidFrameCount = frames;
}
#pragma AVRT_CODE_END
