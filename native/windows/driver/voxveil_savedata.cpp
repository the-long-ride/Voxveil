// Voxveil deliberately replaces SysVAD's sample disk-writing implementation.
// The generic miniport still references CSaveData, but every method here is a
// no-op so render audio can never be persisted by this driver.

#include <sysvad.h>
#include "savedata.h"

ULONG CSaveData::m_ulStreamId = 0;
ULONG CSaveData::m_ulOffloadStreamId = 0;

CSaveData::CSaveData()
    : m_FileHandle(nullptr),
      m_pDataBuffer(nullptr),
      m_ulBufferSize(0),
      m_ulFrameIndex(0),
      m_ulFrameCount(0),
      m_ulFrameSize(0),
      m_ulBufferOffset(0),
      m_fFrameUsed(nullptr),
      m_waveFormat(nullptr),
      m_pFilePtr(nullptr),
      m_pWorkItems(nullptr),
      m_fWriteDisabled(TRUE),
      m_bInitialized(FALSE) {
    RtlZeroMemory(&m_FileName, sizeof(m_FileName));
    RtlZeroMemory(&m_objectAttributes, sizeof(m_objectAttributes));
    RtlZeroMemory(&m_FileHeader, sizeof(m_FileHeader));
    RtlZeroMemory(&m_DataHeader, sizeof(m_DataHeader));
    KeInitializeSpinLock(&m_FrameInUseSpinLock);
    KeInitializeMutex(&m_FileSync, 0);
}

CSaveData::~CSaveData() = default;

NTSTATUS CSaveData::InitializeWorkItems(PDEVICE_OBJECT) { return STATUS_SUCCESS; }
void CSaveData::DestroyWorkItems() {}
void CSaveData::Disable(BOOL) { m_fWriteDisabled = TRUE; }
PSAVEWORKER_PARAM CSaveData::GetNewWorkItem() { return nullptr; }
NTSTATUS CSaveData::Initialize(BOOL) {
    m_bInitialized = TRUE;
    m_fWriteDisabled = TRUE;
    return STATUS_SUCCESS;
}
NTSTATUS CSaveData::SetDeviceObject(PDEVICE_OBJECT deviceObject) {
    m_pDeviceObject = deviceObject;
    return STATUS_SUCCESS;
}
PDEVICE_OBJECT CSaveData::GetDeviceObject() { return m_pDeviceObject; }
void CSaveData::ReadData(PBYTE buffer, ULONG byteCount) {
    if (buffer != nullptr && byteCount != 0) {
        RtlZeroMemory(buffer, byteCount);
    }
}
NTSTATUS CSaveData::SetDataFormat(PKSDATAFORMAT) { return STATUS_SUCCESS; }
NTSTATUS CSaveData::SetMaxWriteSize(ULONG) { return STATUS_SUCCESS; }
void CSaveData::WaitAllWorkItems() {}
void CSaveData::WriteData(PBYTE, ULONG) {}

VOID SaveFrameWorkerCallback(PDEVICE_OBJECT, PVOID) {}
