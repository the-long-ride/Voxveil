#include "VoxveilApo.h"

#include <algorithm>
#include <cstring>

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
    if (state_ != nullptr) {
        InterlockedIncrement(&state_->loadedInstances);
    }
}

CVoxveilApo::~CVoxveilApo() noexcept {
    if (state_ != nullptr) {
        InterlockedDecrement(&state_->loadedInstances);
    }
    voxveil::CloseSharedState(mapping_, state_);
    mapping_ = nullptr;
    state_ = nullptr;
}

STDMETHODIMP CVoxveilApo::Initialize(UINT32 cbDataSize, BYTE* data) {
    if ((data == nullptr && cbDataSize != 0) || (data != nullptr && cbDataSize == 0)) {
        return E_INVALIDARG;
    }
    if (m_bIsInitialized) {
        return APOERR_ALREADY_INITIALIZED;
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
    const UINT32 frames = input->u32ValidFrameCount;
    const UINT32 channels = GetSamplesPerFrame();
    auto* inputSamples = static_cast<FLOAT32*>(input->pBuffer);
    auto* outputSamples = static_cast<FLOAT32*>(output->pBuffer);

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
        if (state_ != nullptr) {
            InterlockedIncrement(&state_->heartbeat);
        }
        return;
    }

    if (outputSamples != inputSamples) {
        CopyMemory(outputSamples, inputSamples, sampleCount * sizeof(FLOAT32));
    }

    if (state_ != nullptr) {
        InterlockedIncrement(&state_->heartbeat);
        const LONG enabled = InterlockedCompareExchange(&state_->enabled, 0, 0);
        const LONG vocal = std::clamp<LONG>(
            InterlockedCompareExchange(&state_->vocalPercent, 0, 0), 0, 100);

        if (enabled != 0 && vocal < 100 && channels >= 2) {
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
