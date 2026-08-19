[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$TopologyGuid = '{DDA54A40-1E4C-11D1-A050-405705C10000}'

function Get-DevicePropertyValue([string]$InstanceId, [string]$KeyName) {
  try {
    return (Get-PnpDeviceProperty -InstanceId $InstanceId -KeyName $KeyName -ErrorAction Stop).Data
  } catch {
    return $null
  }
}

function Join-InfLogicalLines([string[]]$Lines) {
  $logical = [Collections.Generic.List[string]]::new()
  $pending = ''
  foreach ($raw in $Lines) {
    $line = $raw.TrimEnd()
    if ($pending) { $line = $pending + $line.TrimStart() }
    if ($line.EndsWith('\')) {
      $pending = $line.Substring(0, $line.Length - 1)
      continue
    }
    $logical.Add($line)
    $pending = ''
  }
  if ($pending) { $logical.Add($pending) }
  return @($logical)
}

function Read-InfStrings([string[]]$Lines) {
  $strings = @{}
  $inStrings = $false
  foreach ($line in $Lines) {
    if ($line -match '^\s*\[([^\]]+)\]\s*$') {
      $inStrings = $Matches[1] -match '^Strings(?:\.|$)'
      continue
    }
    if (-not $inStrings) { continue }
    $clean = ($line -split ';', 2)[0].Trim()
    if ($clean -match '^([^=]+?)\s*=\s*"?(.*?)"?\s*$') {
      $strings[$Matches[1].Trim()] = $Matches[2].Trim().Trim('"')
    }
  }
  return $strings
}

function Resolve-InfToken([string]$Value, [hashtable]$Strings) {
  $value = $Value.Trim().Trim('"')
  if ($value -match '^%(.+)%$' -and $Strings.ContainsKey($Matches[1])) {
    return [string]$Strings[$Matches[1]]
  }
  return $value
}

function Get-TopologyReferences([string]$InfPath, [string[]]$HardwareIds) {
  if (-not (Test-Path $InfPath -PathType Leaf)) { return @() }
  $lines = Join-InfLogicalLines @(Get-Content $InfPath)
  $strings = Read-InfStrings $lines
  $installSections = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)

  foreach ($line in $lines) {
    $clean = ($line -split ';', 2)[0].Trim()
    if (-not $clean -or $clean.StartsWith('[')) { continue }
    $matchesHardware = $false
    foreach ($hardwareId in $HardwareIds) {
      if ($hardwareId -and $clean.IndexOf($hardwareId, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
        $matchesHardware = $true
        break
      }
    }
    if ($matchesHardware -and $clean -match '^[^=]+?=\s*([^,\s]+)\s*,') {
      [void]$installSections.Add($Matches[1].Trim())
    }
  }

  if ($installSections.Count -eq 0) { return @() }
  $references = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
  $active = $false
  foreach ($line in $lines) {
    if ($line -match '^\s*\[([^\]]+)\]\s*$') {
      $section = $Matches[1].Trim()
      $active = $false
      foreach ($install in $installSections) {
        $pattern = '^' + [Regex]::Escape($install) + '(?:\.[^.\]]+)*\.Interfaces$'
        if ($section -match $pattern) { $active = $true; break }
      }
      continue
    }
    if (-not $active) { continue }
    $clean = ($line -split ';', 2)[0].Trim()
    if ($clean -notmatch '^AddInterface\s*=\s*([^,]+)\s*,\s*([^,]+)') { continue }
    $category = Resolve-InfToken $Matches[1] $strings
    $reference = Resolve-InfToken $Matches[2] $strings
    $isTopology = $category -ieq $TopologyGuid -or $reference -match '(?i)topo'
    if ($isTopology -and $reference) { [void]$references.Add($reference) }
  }
  return @($references | Sort-Object)
}

function Resolve-ParentAudioDevice([string]$EndpointInstanceId) {
  $current = $EndpointInstanceId
  for ($depth = 0; $depth -lt 8; $depth++) {
    $parent = Get-DevicePropertyValue $current 'DEVPKEY_Device_Parent'
    if (-not $parent) { break }
    $parent = [string]$parent
    $hardwareIds = @(Get-DevicePropertyValue $parent 'DEVPKEY_Device_HardwareIds') | Where-Object { $_ }
    $driverInf = Get-DevicePropertyValue $parent 'DEVPKEY_Device_DriverInfPath'
    if ($hardwareIds.Count -gt 0 -and $driverInf) {
      $device = Get-PnpDevice -InstanceId $parent -ErrorAction SilentlyContinue
      return [pscustomobject]@{
        InstanceId = $parent
        AdapterName = if ($device) { [string]$device.FriendlyName } else { $null }
        HardwareIds = @($hardwareIds | ForEach-Object { [string]$_ })
        DriverInf = [string]$driverInf
      }
    }
    $current = $parent
  }
  return $null
}

function Match-EndpointPnpDevice($CoreEndpoint, [object[]]$PnpEndpoints) {
  $id = [string]$CoreEndpoint.endpointId
  $exact = @($PnpEndpoints | Where-Object {
    $_.InstanceId -ieq $id -or $_.InstanceId.EndsWith($id, [StringComparison]::OrdinalIgnoreCase)
  })
  if ($exact.Count -eq 1) { return [pscustomobject]@{ Device = $exact[0]; Detail = $null } }
  if ($exact.Count -gt 1) {
    return [pscustomobject]@{ Device = $null; Detail = 'Multiple AudioEndpoint PnP devices matched the Core Audio endpoint ID.' }
  }

  $byName = @($PnpEndpoints | Where-Object { $_.FriendlyName -ieq [string]$CoreEndpoint.displayName })
  if ($byName.Count -eq 1) { return [pscustomobject]@{ Device = $byName[0]; Detail = $null } }
  if ($byName.Count -gt 1) {
    return [pscustomobject]@{ Device = $null; Detail = 'Multiple AudioEndpoint PnP devices share this playback endpoint name.' }
  }
  return [pscustomobject]@{ Device = $null; Detail = 'No present AudioEndpoint PnP device matched this Core Audio endpoint.' }
}

$inputJson = [Console]::In.ReadToEnd()
if (-not $inputJson.Trim()) { throw 'Expected Core Audio endpoint JSON on stdin.' }
$coreEndpoints = @(ConvertFrom-Json $inputJson | Where-Object { $null -ne $_ })
$pnpEndpoints = @(Get-PnpDevice -Class AudioEndpoint -PresentOnly -ErrorAction SilentlyContinue)
$results = [Collections.Generic.List[object]]::new()

foreach ($endpoint in $coreEndpoints) {
  $match = Match-EndpointPnpDevice $endpoint $pnpEndpoints
  if (-not $match.Device) {
    $results.Add([pscustomobject]@{
      endpointId = [string]$endpoint.endpointId; adapterName = $null; pnpInstanceId = $null
      hardwareIds = @(); driverInf = $null; topologyReferences = @(); detail = $match.Detail
    })
    continue
  }

  $parent = Resolve-ParentAudioDevice ([string]$match.Device.InstanceId)
  if (-not $parent) {
    $results.Add([pscustomobject]@{
      endpointId = [string]$endpoint.endpointId; adapterName = $null; pnpInstanceId = $null
      hardwareIds = @(); driverInf = $null; topologyReferences = @()
      detail = 'The playback endpoint parent chain did not expose hardware IDs and an installed driver INF.'
    })
    continue
  }

  $infPath = Join-Path $env:windir ('INF\' + $parent.DriverInf)
  $topology = @(Get-TopologyReferences $infPath $parent.HardwareIds)
  $detail = if ($topology.Count -gt 1) {
    'Multiple topology references matched the installed audio driver; Voxveil will not guess.'
  } elseif ($topology.Count -eq 0) {
    'No unambiguous topology AddInterface reference was found in the installed audio driver INF.'
  } else { $null }

  $results.Add([pscustomobject]@{
    endpointId = [string]$endpoint.endpointId
    adapterName = $parent.AdapterName
    pnpInstanceId = $parent.InstanceId
    hardwareIds = @($parent.HardwareIds)
    driverInf = $parent.DriverInf
    topologyReferences = @($topology)
    detail = $detail
  })
}

ConvertTo-Json -InputObject @($results) -Depth 5 -Compress
