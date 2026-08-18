#include "voxveil_apo.h"
#include "voxveil_dsp.h"

#include <algorithm>
#include <cstring>

const CLSID CLSID_VoxveilApo = {
    0x7e268e67,
    0x2f3c,
    0x4f0a,
    {0xa0, 0x9c, 0x8b, 0x7d, 0x27, 0xb4, 0x3f, 0x51}};

#pragma warning(disable : 4815)
const AVRT_DATA CRegAPOProperties<1> CVoxveilApo::registration_properties(
    CLSID_VoxveilApo,
    L"Voxveil Endpoint Effect",
    L"Voxveil contributors",
    1,
    0,
    __uuidof(IAudioProcessingObject),
    static_cast<APO_FLAG>(APO_FLAG_INPLACE | APO_FLAG_DEFAULT));

CVoxveilApo::CVoxveilApo()
    : CBaseAudioProcessingObject(registration_properties) {}

STDMETHODIMP CVoxveilApo::Initialize(UINT32 cb_data_size, BYTE* data) {
    if ((data == nullptr && cb_data_size != 0) ||
        (data != nullptr && cb_data_size == 0)) {
        return E_INVALIDARG;
    }
    if (m_bIsInitialized) {
        return APOERR_ALREADY_INITIALIZED;
    }
    if (data != nullptr) {
        if (cb_data_size < sizeof(APOInitBaseStruct)) {
            return E_INVALIDARG;
        }
        const auto* init = reinterpret_cast<const APOInitBaseStruct*>(data);
        if (!IsEqualCLSID(init->clsid, CLSID_VoxveilApo)) {
            return APOERR_INVALID_APO_CLSID;
        }
    }
    m_bIsInitialized = true;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::GetLatency(HNSTIME* latency) {
    if (latency == nullptr) {
        return E_POINTER;
    }
    *latency = 0;
    return S_OK;
}

STDMETHODIMP CVoxveilApo::LockForProcess(
    UINT32 input_count,
    APO_CONNECTION_DESCRIPTOR** inputs,
    UINT32 output_count,
    APO_CONNECTION_DESCRIPTOR** outputs) {
    const HRESULT result = CBaseAudioProcessingObject::LockForProcess(
        input_count, inputs, output_count, outputs);
    if (SUCCEEDED(result)) {
        channels_ = GetSamplesPerFrame();
    }
    return result;
}

#pragma AVRT_CODE_BEGIN
STDMETHODIMP_(void) CVoxveilApo::APOProcess(
    UINT32 input_count,
    APO_CONNECTION_PROPERTY** inputs,
    UINT32 output_count,
    APO_CONNECTION_PROPERTY** outputs) {
    if (input_count == 0 || output_count == 0 || inputs == nullptr || outputs == nullptr ||
        inputs[0] == nullptr || outputs[0] == nullptr) {
        return;
    }

    APO_CONNECTION_PROPERTY* input = inputs[0];
    APO_CONNECTION_PROPERTY* output = outputs[0];
    output->u32BufferFlags = input->u32BufferFlags;
    output->u32ValidFrameCount = input->u32ValidFrameCount;

    if (input->u32BufferFlags != BUFFER_VALID) {
        return;
    }

    auto* input_samples = static_cast<float*>(input->pBuffer);
    auto* output_samples = static_cast<float*>(output->pBuffer);
    if (input_samples == nullptr || output_samples == nullptr) {
        output->u32BufferFlags = BUFFER_INVALID;
        output->u32ValidFrameCount = 0;
        return;
    }

    const std::size_t sample_count =
        static_cast<std::size_t>(input->u32ValidFrameCount) * channels_;
    if (output_samples != input_samples) {
        std::memcpy(output_samples, input_samples, sample_count * sizeof(float));
    }

    voxveil::process_interleaved(
        output_samples,
        input->u32ValidFrameCount,
        channels_,
        control_.enabled(),
        control_.vocal_level());
}
#pragma AVRT_CODE_END
