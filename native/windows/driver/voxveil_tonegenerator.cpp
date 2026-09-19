// Voxveil deliberately replaces SysVAD's sample sine-wave generator.
// The generic WaveRT stream references ToneGenerator, but Voxveil's production
// virtual render endpoint must never synthesize sample audio. This compatible
// implementation accepts initialization and emits silence only.

#include <sysvad.h>
#include "tonegenerator.h"

ToneGenerator::ToneGenerator()
    : m_Frequency(0),
      m_ChannelCount(0),
      m_BitsPerSample(0),
      m_SamplesPerSecond(0),
      m_Theta(0.0),
      m_SampleIncrement(0.0),
      m_Mute(true),
      m_PartialFrame(nullptr),
      m_PartialFrameBytes(0),
      m_FrameSize(0),
      m_ToneAmplitude(0.0),
      m_ToneDCOffset(0.0) {}

ToneGenerator::~ToneGenerator() = default;

NTSTATUS ToneGenerator::Init(
    DWORD toneFrequency,
    double toneAmplitude,
    double toneDCOffset,
    double toneInitialPhase,
    PWAVEFORMATEXTENSIBLE waveFormat) {
    UNREFERENCED_PARAMETER(toneFrequency);
    UNREFERENCED_PARAMETER(toneAmplitude);
    UNREFERENCED_PARAMETER(toneDCOffset);
    UNREFERENCED_PARAMETER(toneInitialPhase);

    m_Frequency = 0;
    m_ToneAmplitude = 0.0;
    m_ToneDCOffset = 0.0;
    m_Theta = 0.0;
    m_SampleIncrement = 0.0;
    m_Mute = true;
    m_PartialFrameBytes = 0;

    if (waveFormat != nullptr) {
        m_ChannelCount = waveFormat->Format.nChannels;
        m_BitsPerSample = waveFormat->Format.wBitsPerSample;
        m_SamplesPerSecond = waveFormat->Format.nSamplesPerSec;
        m_FrameSize = waveFormat->Format.nBlockAlign;
    } else {
        m_ChannelCount = 0;
        m_BitsPerSample = 0;
        m_SamplesPerSecond = 0;
        m_FrameSize = 0;
    }

    return STATUS_SUCCESS;
}

VOID ToneGenerator::GenerateSine(BYTE* buffer, size_t bufferLength) {
    if (buffer != nullptr && bufferLength != 0) {
        RtlZeroMemory(buffer, bufferLength);
    }
}
