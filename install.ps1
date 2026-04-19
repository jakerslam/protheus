param(
  [switch]$Full,
  [switch]$Minimal,
  [switch]$Pure,
  [switch]$TinyMax,
  [switch]$Repair,
  [switch]$Force,
  [string]$InstallDir,
  [string]$TmpDir
)

$ErrorActionPreference = "Stop"

$RepoOwner = "protheuslabs"
$RepoName = "InfRing"
$DefaultApi = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
$DefaultReleasesApi = "https://api.github.com/repos/$RepoOwner/$RepoName/releases?per_page=30"
$DefaultLatestUrl = "https://github.com/$RepoOwner/$RepoName/releases/latest"
$DefaultBase = "https://github.com/$RepoOwner/$RepoName/releases/download"
$ReadmeWindowsInstallCommand = 'Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; $tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; & $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue'

$InstallDir = if ($InstallDir) {
  $InstallDir
} elseif ($env:INFRING_INSTALL_DIR) {
  $env:INFRING_INSTALL_DIR
} else {
  Join-Path $HOME ".infring\bin"
}
$InstallDirExplicit = $PSBoundParameters.ContainsKey("InstallDir")
$legacyInstallDir = Join-Path $HOME ".protheus\bin"
$canonicalInstallDir = Join-Path $HOME ".infring\bin"
$normalizedInstallDir = if ($InstallDir) { $InstallDir.TrimEnd("\", "/").ToLower() } else { "" }
$normalizedLegacyInstallDir = $legacyInstallDir.TrimEnd("\", "/").ToLower()
if (
  (-not $InstallDirExplicit) -and
  $InstallDir -and
  (
    $normalizedInstallDir -eq $normalizedLegacyInstallDir -or
    $normalizedInstallDir.EndsWith("\\.protheus\\bin") -or
    $normalizedInstallDir.EndsWith("/.protheus/bin")
  )
) {
  Write-Host "[infring install] detected legacy compatibility install dir ($InstallDir); migrating to canonical $canonicalInstallDir"
  $InstallDir = $canonicalInstallDir
}
$TmpDir = if ($TmpDir) {
  $TmpDir
} elseif ($env:INFRING_TMP_DIR) {
  $env:INFRING_TMP_DIR
} else {
  $null
}
$RequestedVersion = if ($env:INFRING_VERSION) { $env:INFRING_VERSION } else { "latest" }
$ApiUrl = if ($env:INFRING_RELEASE_API_URL) { $env:INFRING_RELEASE_API_URL } else { $DefaultApi }
$ReleasesApiUrl = if ($env:INFRING_RELEASES_API_URL) { $env:INFRING_RELEASES_API_URL } else { $DefaultReleasesApi }
$LatestUrl = if ($env:INFRING_RELEASE_LATEST_URL) { $env:INFRING_RELEASE_LATEST_URL } else { $DefaultLatestUrl }
$BaseUrl = if ($env:INFRING_RELEASE_BASE_URL) { $env:INFRING_RELEASE_BASE_URL } else { $DefaultBase }
$InstallFull = $false
if ($env:INFRING_INSTALL_FULL -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_FULL.ToLower()) {
  $InstallFull = $true
}
$InstallPure = $false
if ($env:INFRING_INSTALL_PURE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_PURE.ToLower()) {
  $InstallPure = $true
}
$InstallTinyMax = $false
if ($env:INFRING_INSTALL_TINY_MAX -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_TINY_MAX.ToLower()) {
  $InstallTinyMax = $true
}
$InstallRepair = $false
if ($env:INFRING_INSTALL_REPAIR -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_REPAIR.ToLower()) {
  $InstallRepair = $true
}
if ($Full) { $InstallFull = $true }
if ($Minimal) { $InstallFull = $false }
if ($Pure) {
  $InstallPure = $true
  $InstallFull = $false
}
if ($TinyMax) {
  $InstallTinyMax = $true
  $InstallPure = $true
  $InstallFull = $false
}
if ($Repair) { $InstallRepair = $true }
if ($Force) {
  # Compatibility shim for operators accustomed to `-Force`.
  # Treat this as an explicit repair pass and bias to `-Full` unless the caller
  # already selected a constrained mode.
  $InstallRepair = $true
  if (-not ($Minimal -or $Pure -or $TinyMax)) {
    $InstallFull = $true
  }
}

if ($TmpDir) {
  New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null
  $env:TMPDIR = $TmpDir
  $env:TEMP = $TmpDir
  $env:TMP = $TmpDir
}

$script:SourceFallbackDir = $null
$script:SourceFallbackTmp = $null
$script:LastBinaryInstallFailure = $null
$script:LastBinaryInstallFailureReason = ""
$script:WindowsInstallPreflight = $null

function Installer-TruthyFlag([string]$RawValue, [bool]$DefaultValue = $false) {
  if ([string]::IsNullOrWhiteSpace($RawValue)) {
    return $DefaultValue
  }
  return @("1", "true", "yes", "on") -contains $RawValue.ToLower()
}

function Install-AutoRustupEnabled {
  return Installer-TruthyFlag $env:INFRING_INSTALL_AUTO_RUSTUP $true
}

function Resolve-Arch {
  $archRaw = if ($env:PROCESSOR_ARCHITECTURE) { $env:PROCESSOR_ARCHITECTURE } else { [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() }
  switch ($archRaw.ToLower()) {
    "amd64" { "x86_64" }
    "arm64" { "aarch64" }
    default { throw "Unsupported architecture: $archRaw" }
  }
}

function Resolve-HostOsFlags {
  $runtime = [System.Runtime.InteropServices.RuntimeInformation]
  $osPlatform = [System.Runtime.InteropServices.OSPlatform]
  $isWindowsRuntime = $runtime::IsOSPlatform($osPlatform::Windows)
  $isLinuxRuntime = $runtime::IsOSPlatform($osPlatform::Linux)
  $isMacRuntime = $runtime::IsOSPlatform($osPlatform::OSX)

  # PowerShell 6+ exposes $IsWindows/$IsLinux/$IsMacOS.
  # Windows PowerShell 5.1 does not, so runtime probing must remain the source of truth.
  $isWindows = if (Get-Variable -Name IsWindows -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsWindows -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isWindowsRuntime
  }
  $isLinux = if (Get-Variable -Name IsLinux -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsLinux -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isLinuxRuntime
  }
  $isMacOS = if (Get-Variable -Name IsMacOS -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsMacOS -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isMacRuntime
  }

  if (-not ($isWindows -or $isLinux -or $isMacOS)) {
    $platformDescription = [string]$runtime::OSDescription
    throw "Unsupported OS for installer (detected: $platformDescription)"
  }

  return @{
    IsWindows = $isWindows
    IsLinux = $isLinux
    IsMacOS = $isMacOS
  }
}

function Normalize-WindowsPathEntry([string]$value) {
  if ([string]::IsNullOrWhiteSpace($value)) {
    return ""
  }
  $trimmed = $value.Trim().Trim('"')
  if ($trimmed.EndsWith("\")) {
    $trimmed = $trimmed.TrimEnd('\')
  }
  return $trimmed.ToLowerInvariant()
}

function Ensure-WindowsPathContains([string]$pathValue, [string]$entry, [switch]$PreferFront, [string[]]$RemoveEntries = @()) {
  $parts = @()
  if (-not [string]::IsNullOrWhiteSpace($pathValue)) {
    $parts = $pathValue.Split(";") |
      ForEach-Object { [string]$_ } |
      ForEach-Object { $_.Trim().Trim('"') } |
      Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  }

  $entryClean = [string]$entry
  $entryNorm = Normalize-WindowsPathEntry $entryClean
  $removeNorms = @{}
  foreach ($removeEntry in $RemoveEntries) {
    $removeNorm = Normalize-WindowsPathEntry $removeEntry
    if (-not [string]::IsNullOrWhiteSpace($removeNorm)) {
      $removeNorms[$removeNorm] = $true
    }
  }
  $seen = @{}
  $deduped = New-Object System.Collections.Generic.List[string]
  $containsEntry = $false

  foreach ($part in $parts) {
    $norm = Normalize-WindowsPathEntry $part
    if ([string]::IsNullOrWhiteSpace($norm)) {
      continue
    }
    if ($removeNorms.ContainsKey($norm) -and $norm -ne $entryNorm) {
      continue
    }
    if ($norm -eq $entryNorm) {
      $containsEntry = $true
      if ($PreferFront) {
        continue
      }
    }
    if (-not $seen.ContainsKey($norm)) {
      $deduped.Add($part)
      $seen[$norm] = $true
    }
  }

  if ($PreferFront) {
    $deduped.Insert(0, $entryClean)
  } elseif (-not $containsEntry) {
    $deduped.Add($entryClean)
  }

  $joined = ($deduped -join ";")
  return @{
    Value = $joined
    Added = (-not $containsEntry)
    Changed = ($joined -ne [string]$pathValue)
  }
}

function Invoke-SourceFallbackCleanup {
  if (-not ($script:SourceFallbackTmp -and (Test-Path $script:SourceFallbackTmp.FullName))) {
    return
  }

  $cleanupRoot = $script:SourceFallbackTmp.FullName
  $script:SourceFallbackTmp = $null
  $script:SourceFallbackDir = $null

  if ($HostIsWindows) {
    try {
      Start-Process -FilePath "cmd.exe" -ArgumentList @("/d", "/c", "rmdir /s /q `"$cleanupRoot`"") -WindowStyle Hidden | Out-Null
      Write-Host "[infring install] scheduled background cleanup of source fallback temp dir: $cleanupRoot"
      return
    } catch {
      Write-Host "[infring install] warning: background temp cleanup scheduling failed; falling back to synchronous cleanup"
    }
  }

  Remove-Item -Force -Recurse $cleanupRoot
}

function Resolve-Version {
  function Normalize-Version([string]$RawVersion) {
    if ($RawVersion.StartsWith("v")) { return $RawVersion }
    return "v$RawVersion"
  }

  function Resolve-VersionFromApi {
    try {
      $release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
      if ($release.tag_name) {
        return Normalize-Version ([string]$release.tag_name)
      }
    } catch {
      return $null
    }
    return $null
  }

  function Resolve-VersionFromRedirect {
    try {
      $response = Invoke-WebRequest -Uri $LatestUrl -Method Head -MaximumRedirection 10 -UseBasicParsing
      $finalUrl = $response.BaseResponse.ResponseUri.AbsoluteUri
      if (-not $finalUrl) { return $null }
      if ($finalUrl -match "/releases/tag/(v[^/?#]+)") {
        return $Matches[1]
      }
    } catch {
      return $null
    }
    return $null
  }

  if ($RequestedVersion -ne "latest") {
    return Normalize-Version $RequestedVersion
  }

  $version = Resolve-VersionFromApi
  if ($version) { return $version }

  $version = Resolve-VersionFromRedirect
  if ($version) {
    Write-Host "[infring install] GitHub API unavailable; resolved latest tag via releases/latest redirect: $version"
    return $version
  }

  $fallback = if ($env:INFRING_FALLBACK_VERSION) { $env:INFRING_FALLBACK_VERSION } else { $null }
  if ($fallback) {
    $fallbackVersion = Normalize-Version ([string]$fallback)
    Write-Host "[infring install] using fallback version: $fallbackVersion"
    return $fallbackVersion
  }

  throw "Failed to resolve latest release tag (GitHub API + releases/latest redirect). Set INFRING_VERSION=vX.Y.Z and retry."
}

function Get-ReleasesFromApi {
  try {
    $releases = Invoke-RestMethod -Uri $ReleasesApiUrl -UseBasicParsing
    if ($releases -is [System.Array]) {
      return @($releases)
    }
    return @()
  } catch {
    return @()
  }
}

function Get-BinaryStemAliases([string]$Stem) {
  switch ($Stem) {
    "infring-ops" { return @("infring-ops", "protheus-ops") }
    "infringd" { return @("infringd", "protheusd") }
    "infringd-tiny-max" { return @("infringd-tiny-max", "protheusd-tiny-max", "infringd", "protheusd") }
    "infring-pure-workspace" { return @("infring-pure-workspace", "protheus-pure-workspace") }
    "infring-pure-workspace-tiny-max" { return @("infring-pure-workspace-tiny-max", "protheus-pure-workspace-tiny-max", "infring-pure-workspace", "protheus-pure-workspace") }
    default { return @($Stem) }
  }
}

function Get-BinaryAssetCandidates([string]$Triple, [string]$Stem) {
  $variants = New-Object System.Collections.Generic.List[string]
  foreach ($alias in (Get-BinaryStemAliases $Stem)) {
    foreach ($candidate in @(
      "$alias-$Triple.exe",
      "$alias-$Triple",
      "$alias-$Triple.bin",
      "$alias.exe",
      "$alias"
    )) {
      if (-not $variants.Contains([string]$candidate)) {
        $variants.Add([string]$candidate) | Out-Null
      }
    }
  }
  return @($variants)
}

function Release-HasAnyAsset([object]$Release, [string[]]$AssetCandidates) {
  if (-not $Release) { return $false }
  $assets = @()
  if ($Release.assets -is [System.Array]) {
    $assets = @($Release.assets | ForEach-Object { [string]$_.name })
  }
  if ($assets.Count -eq 0) { return $false }
  foreach ($candidate in $AssetCandidates) {
    if ($assets -contains $candidate) {
      return $true
    }
  }
  return $false
}

function Resolve-AssetCompatibleVersionForTriple([string]$Triple, [string[]]$Stems) {
  if ($RequestedVersion -ne "latest") {
    return $null
  }
  $releases = Get-ReleasesFromApi
  if ($releases.Count -eq 0) {
    return $null
  }
  foreach ($release in $releases) {
    if (-not $release) { continue }
    if ([bool]$release.draft) { continue }
    if (-not $release.tag_name) { continue }
    $allPresent = $true
    foreach ($stem in $Stems) {
      $assetCandidates = Get-BinaryAssetCandidates $Triple $stem
      if (-not (Release-HasAnyAsset $release $assetCandidates)) {
        $allPresent = $false
        break
      }
    }
    if ($allPresent) {
      return [string]$release.tag_name
    }
  }
  return $null
}

function Resolve-ReleaseByTag([string]$VersionTag) {
  if ([string]::IsNullOrWhiteSpace($VersionTag)) {
    return $null
  }
  $releases = Get-ReleasesFromApi
  if ($releases.Count -eq 0) {
    return $null
  }
  $normalized = [string]$VersionTag
  foreach ($release in $releases) {
    if (-not $release) { continue }
    $tag = [string]$release.tag_name
    if ([string]::IsNullOrWhiteSpace($tag)) { continue }
    if ($tag -eq $normalized -or $tag.TrimStart("v") -eq $normalized.TrimStart("v")) {
      return $release
    }
  }
  return $null
}

function Probe-ReleaseAssetReachability([string]$VersionTag, [string]$AssetName) {
  $url = "$BaseUrl/$VersionTag/$AssetName"
  try {
    Invoke-WebRequest -Uri $url -Method Head -UseBasicParsing -TimeoutSec 20 | Out-Null
    return @{
      reachable = $true
      status = "head_ok"
      url = $url
    }
  } catch {
    try {
      Invoke-WebRequest -Uri $url -Method Get -Headers @{ Range = "bytes=0-0" } -UseBasicParsing -TimeoutSec 20 | Out-Null
      return @{
        reachable = $true
        status = "range_get_ok"
        url = $url
      }
    } catch {
      $status = "request_failed"
      try {
        $status = [string][int]$_.Exception.Response.StatusCode.value__
      } catch {
      }
      return @{
        reachable = $false
        status = $status
        url = $url
      }
    }
  }
}

function Resolve-ReleaseAssetProbe([string]$VersionTag, [string]$Triple, [string]$Stem) {
  $release = Resolve-ReleaseByTag $VersionTag
  $candidates = Get-BinaryAssetCandidates $Triple $Stem
  if (-not $release) {
    return @{
      stem = $Stem
      version = $VersionTag
      selected_asset = ""
      asset_found = $false
      reachable = $false
      reachability_status = "release_metadata_unavailable"
      candidates = $candidates
    }
  }
  $assetNames = @()
  if ($release.assets -is [System.Array]) {
    $assetNames = @($release.assets | ForEach-Object { [string]$_.name })
  }
  $selected = ""
  foreach ($candidate in $candidates) {
    if ($assetNames -contains $candidate) {
      $selected = $candidate
      break
    }
  }
  if ([string]::IsNullOrWhiteSpace($selected)) {
    return @{
      stem = $Stem
      version = $VersionTag
      selected_asset = ""
      asset_found = $false
      reachable = $false
      reachability_status = "asset_not_listed_in_release"
      candidates = $candidates
    }
  }
  $reachability = Probe-ReleaseAssetReachability $VersionTag $selected
  return @{
    stem = $Stem
    version = $VersionTag
    selected_asset = $selected
    asset_found = $true
    reachable = [bool]$reachability.reachable
    reachability_status = [string]$reachability.status
    reachability_url = [string]$reachability.url
    candidates = $candidates
  }
}

function Get-WindowsBuildToolSummary {
  $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
  $rustcCmd = Get-Command rustc -ErrorAction SilentlyContinue
  $clCmd = Get-Command cl.exe -ErrorAction SilentlyContinue
  $vswhereCmd = Get-Command vswhere.exe -ErrorAction SilentlyContinue
  $vsInstallDetected = $false
  if ($vswhereCmd) {
    try {
      $vsPath = & $vswhereCmd.Source -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
      if (-not [string]::IsNullOrWhiteSpace([string]$vsPath)) {
        $vsInstallDetected = $true
      }
    } catch {
    }
  }
  return @{
    cargo_present = [bool]$cargoCmd
    rustc_present = [bool]$rustcCmd
    cl_present = [bool]$clCmd
    vs_install_detected = [bool]$vsInstallDetected
    msvc_tools_present = [bool]$clCmd -or [bool]$vsInstallDetected
  }
}

function Invoke-WindowsInstallerPreflight([string]$VersionTag, [string]$Triple, [string[]]$RequiredStems) {
  if (-not $HostIsWindows) {
    return
  }
  $dedupedStems = @($RequiredStems | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
  if ($dedupedStems.Count -eq 0) {
    return
  }
  $toolchain = Get-WindowsBuildToolSummary
  $assetProbes = @()
  foreach ($stem in $dedupedStems) {
    $assetProbes += Resolve-ReleaseAssetProbe $VersionTag $Triple $stem
  }
  $script:WindowsInstallPreflight = @{
    version = $VersionTag
    triple = $Triple
    required_stems = $dedupedStems
    toolchain = $toolchain
    assets = $assetProbes
  }
  Write-Host ("[infring install] preflight windows toolchain: cargo={0}; rustc={1}; msvc_tools={2}" -f `
      ([string]$toolchain.cargo_present).ToLower(), `
      ([string]$toolchain.rustc_present).ToLower(), `
      ([string]$toolchain.msvc_tools_present).ToLower())
  foreach ($probe in $assetProbes) {
    if ([bool]$probe.asset_found) {
      Write-Host ("[infring install] preflight asset probe ({0}): found {1}; reachable={2} ({3})" -f `
          [string]$probe.stem, `
          [string]$probe.selected_asset, `
          ([string][bool]$probe.reachable).ToLower(), `
          [string]$probe.reachability_status)
    } else {
      Write-Host ("[infring install] preflight asset probe ({0}): missing prebuilt in release metadata ({1})" -f `
          [string]$probe.stem, `
          [string]$probe.reachability_status)
    }
  }
  $assetGaps = @($assetProbes | Where-Object {
      (-not [bool]$_.asset_found) -or
      (([bool]$_.asset_found) -and (-not [bool]$_.reachable))
    })
  $autoRustup = Install-AutoRustupEnabled
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.cargo_present) -and (-not $autoRustup)) {
    if ($RequestedVersion -eq "latest") {
      Write-Host "[infring install] preflight warning: current latest tag has Windows asset gaps and source fallback prerequisites are limited; installer will still try compatible-tag fallback before failing."
      return
    }
    $gapSummary = ($assetGaps | ForEach-Object { [string]$_.stem }) -join ", "
    throw "Windows installer preflight failed: prebuilt asset gaps detected for [$gapSummary], Cargo is unavailable, and auto Rust bootstrap is disabled (INFRING_INSTALL_AUTO_RUSTUP=0). Install Rust + MSVC build tools or publish missing Windows release assets."
  }
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.msvc_tools_present)) {
    Write-Host "[infring install] preflight warning: MSVC build tools were not detected; source fallback may fail if Windows prebuilt assets are unavailable."
  }
}

function Format-BinaryInstallFailureHint([string]$Stem, [string]$Triple, [string]$VersionTag) {
  $parts = New-Object System.Collections.Generic.List[string]
  $failure = $script:LastBinaryInstallFailure
  if ($failure -and ([string]$failure.stem -eq [string]$Stem)) {
    if ($failure.asset_probe) {
      $assetProbe = $failure.asset_probe
      if ([bool]$assetProbe.asset_found) {
        $parts.Add(("asset_probe={0};reachable={1};status={2}" -f `
            [string]$assetProbe.selected_asset, `
            ([string][bool]$assetProbe.reachable).ToLower(), `
            [string]$assetProbe.reachability_status))
      } else {
        $parts.Add(("asset_probe=missing;status={0}" -f [string]$assetProbe.reachability_status))
      }
    }
    $attemptedAssets = @($failure.attempted_assets)
    if ($attemptedAssets.Count -gt 0) {
      $parts.Add(("attempted_assets={0}" -f ($attemptedAssets -join ",")))
    }
    $parts.Add(("source_fallback_attempted={0}" -f ([string][bool]$failure.source_fallback_attempted).ToLower()))
    $sourceFallbackVersions = @($failure.source_fallback_versions)
    if ($sourceFallbackVersions.Count -gt 0) {
      $parts.Add(("source_fallback_versions={0}" -f ($sourceFallbackVersions -join ",")))
    }
    if (-not [string]::IsNullOrWhiteSpace([string]$failure.source_fallback_reason)) {
      $parts.Add(("source_fallback_reason={0}" -f [string]$failure.source_fallback_reason))
    }
  }
  if ($HostIsWindows -and $script:WindowsInstallPreflight) {
    $toolchain = $script:WindowsInstallPreflight.toolchain
    if ($toolchain) {
      $parts.Add(("toolchain:cargo={0};rustc={1};msvc_tools={2}" -f `
          ([string][bool]$toolchain.cargo_present).ToLower(), `
          ([string][bool]$toolchain.rustc_present).ToLower(), `
          ([string][bool]$toolchain.msvc_tools_present).ToLower()))
    }
  }
  if ($parts.Count -eq 0) {
    return "No additional diagnostics captured."
  }
  return ($parts -join " | ")
}

function Download-Asset($Version, $Asset, $OutPath) {
  $url = "$BaseUrl/$Version/$Asset"
  try {
    Invoke-WebRequest -Uri $url -OutFile $OutPath -UseBasicParsing | Out-Null
    Write-Host "[infring install] downloaded $Asset"
    return $true
  } catch {
    return $false
  }
}

function Install-Binary($Version, $Triple, $Stem, $OutPath) {
  function Resolve-SourceBinName([string]$StemName) {
    switch ($StemName) {
      "infring-ops" { return "infring-ops" }
      "infringd" { return "infringd" }
      "infringd-tiny-max" { return "infringd" }
      "conduit_daemon" { return "conduit_daemon" }
      "infring-pure-workspace" { return "infring-pure-workspace" }
      "infring-pure-workspace-tiny-max" { return "infring-pure-workspace" }
      default { return $null }
    }
  }

  function Ensure-CargoToolchainForSourceFallback {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
      return $true
    }
    $script:LastBinaryInstallFailureReason = "cargo_missing"
    if (-not $HostIsWindows) {
      $script:LastBinaryInstallFailureReason = "cargo_missing_non_windows_source_fallback_unavailable"
      return $false
    }
    $autoRustup = Install-AutoRustupEnabled
    if (-not $autoRustup) {
      $script:LastBinaryInstallFailureReason = "cargo_missing_auto_rustup_disabled"
      return $false
    }
    Write-Host "[infring install] prebuilt binary not available; attempting Rust toolchain bootstrap for source fallback"
    $rustupExe = Join-Path ([System.IO.Path]::GetTempPath()) "rustup-init.exe"
    try {
      Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupExe -UseBasicParsing | Out-Null
      $proc = Start-Process -FilePath $rustupExe -ArgumentList "-y --profile minimal --default-toolchain stable" -Wait -PassThru
      if ($proc.ExitCode -ne 0) {
        $script:LastBinaryInstallFailureReason = "rustup_bootstrap_failed"
        return $false
      }
      $cargoBin = Join-Path $HOME ".cargo\bin"
      if (Test-Path $cargoBin) {
        if (-not $env:Path.ToLower().Contains($cargoBin.ToLower())) {
          $env:Path = "$cargoBin;$env:Path"
        }
      }
      $cargoPresent = [bool](Get-Command cargo -ErrorAction SilentlyContinue)
      if (-not $cargoPresent) {
        $script:LastBinaryInstallFailureReason = "cargo_still_missing_after_rustup"
      }
      return $cargoPresent
    } catch {
      $script:LastBinaryInstallFailureReason = "rustup_bootstrap_transport_error"
      return $false
    }
  }

  function Prepare-SourceFallbackRepo([string]$VersionTag) {
    if ($script:SourceFallbackDir -and (Test-Path $script:SourceFallbackDir)) {
      return $script:SourceFallbackDir
    }
    if (-not (Ensure-CargoToolchainForSourceFallback)) {
      return $null
    }

    $script:SourceFallbackTmp = New-TemporaryFile
    Remove-Item $script:SourceFallbackTmp.FullName -Force
    New-Item -ItemType Directory -Path $script:SourceFallbackTmp.FullName | Out-Null
    $script:SourceFallbackDir = Join-Path $script:SourceFallbackTmp.FullName "repo"
    $repoUrl = "https://github.com/$RepoOwner/$RepoName.git"

    $archivePath = Join-Path $script:SourceFallbackTmp.FullName "source.zip"
    $extractRoot = Join-Path $script:SourceFallbackTmp.FullName "extract"
    New-Item -ItemType Directory -Path $extractRoot | Out-Null
    $archiveUrls = @(
      "https://github.com/$RepoOwner/$RepoName/archive/refs/tags/$VersionTag.zip",
      "https://github.com/$RepoOwner/$RepoName/archive/refs/heads/main.zip"
    )
    foreach ($archiveUrl in $archiveUrls) {
      try {
        Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath -UseBasicParsing | Out-Null
        Expand-Archive -Path $archivePath -DestinationPath $extractRoot -Force
        $sourceDir = Get-ChildItem -Path $extractRoot -Directory | Select-Object -First 1
        if ($sourceDir) {
          Copy-Item -Recurse -Force (Join-Path $sourceDir.FullName "*") $script:SourceFallbackDir
          return $script:SourceFallbackDir
        }
      } catch {
      }
    }

    if (Get-Command git -ErrorAction SilentlyContinue) {
      try {
        git clone --quiet --depth 1 --branch main $repoUrl $script:SourceFallbackDir 2>$null | Out-Null
        if (-not [string]::IsNullOrWhiteSpace($VersionTag) -and $VersionTag -ne "main") {
          try {
            git -C $script:SourceFallbackDir fetch --quiet --depth 1 origin ("refs/tags/$VersionTag^{}") 2>$null | Out-Null
            git -c advice.detachedHead=false -C $script:SourceFallbackDir checkout --quiet --detach FETCH_HEAD 2>$null | Out-Null
          } catch {
            try {
              git -c advice.detachedHead=false -C $script:SourceFallbackDir checkout --quiet --detach $VersionTag 2>$null | Out-Null
            } catch {
            }
          }
        }
        return $script:SourceFallbackDir
      } catch {
      }
    }

    $script:LastBinaryInstallFailureReason = "source_repo_unavailable"
    return $null
  }

  function Install-BinaryFromSourceFallback([string]$VersionTag, [string]$StemName, [string]$OutBinaryPath) {
    $binName = Resolve-SourceBinName $StemName
    if (-not $binName) {
      $script:LastBinaryInstallFailureReason = "unsupported_stem_for_source_fallback"
      return $false
    }

    $repoDir = Prepare-SourceFallbackRepo $VersionTag
    if (-not $repoDir) {
      if ([string]::IsNullOrWhiteSpace($script:LastBinaryInstallFailureReason)) {
        $script:LastBinaryInstallFailureReason = "source_repo_prepare_failed"
      }
      return $false
    }

    $manifest = Join-Path $repoDir "core/layer0/ops/Cargo.toml"
    try {
      & cargo build --release --manifest-path $manifest --bin $binName | Out-Null
    } catch {
      $script:LastBinaryInstallFailureReason = "cargo_build_failed"
      return $false
    }
    if ($LASTEXITCODE -ne 0) {
      $script:LastBinaryInstallFailureReason = "cargo_build_failed_exit_$LASTEXITCODE"
      return $false
    }

    $built = Join-Path $repoDir "target/release/$binName.exe"
    if (-not (Test-Path $built)) {
      $script:LastBinaryInstallFailureReason = "source_build_output_missing"
      return $false
    }
    Copy-Item -Force $built $OutBinaryPath
    Write-Host "[infring install] built $binName from source fallback"
    $script:LastBinaryInstallFailureReason = ""
    return $true
  }

  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null

  $assetProbe = Resolve-ReleaseAssetProbe $Version $Triple $Stem
  $attemptedAssets = New-Object System.Collections.Generic.List[string]
  $raw = Join-Path $tmp.FullName "$Stem.exe"
  $assetCandidates = Get-BinaryAssetCandidates $Triple $Stem
  foreach ($assetName in $assetCandidates) {
    $attemptedAssets.Add([string]$assetName)
    if (Download-Asset $Version $assetName $raw) {
      Move-Item -Force $raw $OutPath
      $script:LastBinaryInstallFailure = @{
        stem = $Stem
        triple = $Triple
        version = $Version
        attempted_assets = @($attemptedAssets)
        source_fallback_attempted = $false
        source_fallback_reason = ""
        asset_probe = $assetProbe
      }
      return $true
    }
  }

  $allowNoMsvcSourceFallback = Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK $false
  if (
    $HostIsWindows -and
    $script:WindowsInstallPreflight -and
    (-not [bool]$script:WindowsInstallPreflight.toolchain.msvc_tools_present) -and
    (-not $allowNoMsvcSourceFallback) -and
    (
      (-not [bool]$assetProbe.asset_found) -or
      (([bool]$assetProbe.asset_found) -and (-not [bool]$assetProbe.reachable))
    )
  ) {
    $script:LastBinaryInstallFailureReason = "msvc_tools_missing_no_reachable_prebuilt_asset"
    $script:LastBinaryInstallFailure = @{
      stem = $Stem
      triple = $Triple
      version = $Version
      attempted_assets = @($attemptedAssets)
      source_fallback_attempted = $false
      source_fallback_versions = @()
      source_fallback_reason = [string]$script:LastBinaryInstallFailureReason
      asset_probe = $assetProbe
    }
    return $false
  }

  $script:LastBinaryInstallFailureReason = ""
  $sourceFallbackVersions = @()
  $sourceFallbackPlan = New-Object System.Collections.Generic.List[string]
  $preferMainSourceFallback = (
    ($RequestedVersion -eq "latest") -and
    ($Version -ne "main") -and
    $assetProbe -and
    (-not [bool]$assetProbe.asset_found)
  )
  if ($preferMainSourceFallback) {
    $sourceFallbackPlan.Add("main") | Out-Null
    $sourceFallbackPlan.Add([string]$Version) | Out-Null
  } else {
    $sourceFallbackPlan.Add([string]$Version) | Out-Null
    if (
      ($RequestedVersion -eq "latest") -and
      ($Version -ne "main")
    ) {
      $sourceFallbackPlan.Add("main") | Out-Null
    }
  }
  $fallbackOk = $false
  foreach ($sourceFallbackVersion in $sourceFallbackPlan) {
    $sourceFallbackVersions += [string]$sourceFallbackVersion
    if (
      $preferMainSourceFallback -and
      ($sourceFallbackVersion -eq "main")
    ) {
      Write-Host "[infring install] source fallback using main first (missing prebuilt asset metadata for $Stem on $Triple)"
    } elseif (
      ($sourceFallbackVersion -eq "main") -and
      ($sourceFallbackVersions.Count -gt 1)
    ) {
      Write-Host "[infring install] source fallback for release $Version failed ($script:LastBinaryInstallFailureReason); retrying from main branch"
    }
    $fallbackOk = Install-BinaryFromSourceFallback $sourceFallbackVersion $Stem $OutPath
    if ($fallbackOk) {
      break
    }
  }
  $script:LastBinaryInstallFailure = @{
    stem = $Stem
    triple = $Triple
    version = $Version
    attempted_assets = @($attemptedAssets)
    source_fallback_attempted = $true
    source_fallback_versions = @($sourceFallbackVersions)
    source_fallback_reason = [string]$script:LastBinaryInstallFailureReason
    asset_probe = $assetProbe
  }
  return $fallbackOk
}

function Install-ClientBundle($Version, $Triple, $OutDir) {
  New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null
  $archive = Join-Path $tmp.FullName "client-runtime.bundle"
  function Expand-ClientArchive($ArchivePath, $Destination, $AssetName = $null) {
    if (-not $AssetName) { $AssetName = $ArchivePath }
    if ($AssetName.EndsWith(".tar.zst")) {
      try {
        tar -xf $ArchivePath -C $Destination
        return $true
      } catch {
        if (Get-Command zstd -ErrorAction SilentlyContinue) {
          $tarPath = [System.IO.Path]::ChangeExtension($ArchivePath, ".tar")
          zstd -d --stdout $ArchivePath > $tarPath
          tar -xf $tarPath -C $Destination
          return $true
        }
        Write-Host "[infring install] skipping .tar.zst bundle (zstd unavailable); falling back to .tar.gz assets"
        return $false
      }
    }
    if ($AssetName.EndsWith(".tar.gz")) {
      tar -xzf $ArchivePath -C $Destination
      return $true
    }
    try {
      tar -xzf $ArchivePath -C $Destination
      return $true
    } catch {
      if (Get-Command zstd -ErrorAction SilentlyContinue) {
        $tarPath = [System.IO.Path]::ChangeExtension($ArchivePath, ".tar")
        zstd -d --stdout $ArchivePath > $tarPath
        tar -xf $tarPath -C $Destination
        return $true
      }
    }
    return $false
  }
  $assets = @(
    "infring-client-runtime-$Triple.tar.zst",
    "infring-client-runtime.tar.zst",
    "infring-client-$Triple.tar.zst",
    "infring-client.tar.zst",
    "infring-client-runtime-$Triple.tar.gz",
    "infring-client-runtime.tar.gz",
    "infring-client-$Triple.tar.gz",
    "infring-client.tar.gz"
  )
  foreach ($asset in $assets) {
    if (Download-Asset $Version $asset $archive) {
      if (Expand-ClientArchive $archive $OutDir $asset) {
        Write-Host "[infring install] installed optional client runtime bundle"
        return $true
      }
    }
  }
  return $false
}

function Install-ClientBundleFromSourceFallback($OutDir) {
  if (-not ($script:SourceFallbackDir -and (Test-Path $script:SourceFallbackDir))) {
    return $false
  }

  $repoDir = $script:SourceFallbackDir
  $runtimeSource = Join-Path $repoDir "client/runtime"
  if (-not (Test-Path $runtimeSource)) {
    return $false
  }

  if (Test-Path $OutDir) {
    Remove-Item -Force -Recurse $OutDir
  }
  $clientRoot = Join-Path $OutDir "client"
  New-Item -ItemType Directory -Force -Path $clientRoot | Out-Null
  Copy-Item -Recurse -Force $runtimeSource (Join-Path $clientRoot "runtime")
  Write-Host "[infring install] installed client runtime from source fallback"
  return $true
}

function Resolve-WorkspaceRootForRepair {
  $candidates = @(
    $env:INFRING_WORKSPACE_ROOT,
    # Legacy compatibility only; canonical workspace root env is INFRING_WORKSPACE_ROOT.
    $env:PROTHEUS_WORKSPACE_ROOT,
    (Get-Location).Path,
    (Join-Path $HOME ".infring/workspace"),
    # Legacy compatibility path.
    (Join-Path $HOME ".protheus/workspace")
  )
  foreach ($candidate in $candidates) {
    if (-not $candidate) { continue }
    $manifest = Join-Path $candidate "core/layer0/ops/Cargo.toml"
    $runtimeDir = Join-Path $candidate "client/runtime"
    if ((Test-Path $manifest) -and (Test-Path $runtimeDir)) {
      return $candidate
    }
  }
  return $null
}

function Invoke-RepairInstallDir {
  $targets = @(
    "infring.cmd", "infringctl.cmd", "infringd.cmd",
    # Legacy compatibility wrappers/artifacts (removed during repair migration).
    "protheus.cmd", "protheusctl.cmd", "protheusd.cmd",
    "infring-ops.exe", "infring-pure-workspace.exe",
    "infringd.exe", "conduit_daemon.exe", "infring-client",
    "protheus-ops.exe", "protheus-pure-workspace.exe",
    "protheusd.exe", "protheus-client"
  )
  foreach ($target in $targets) {
    $path = Join-Path $InstallDir $target
    if (Test-Path $path) {
      Remove-Item -Force -Recurse $path
      Write-Host "[infring install] repair removed stale install artifact: $path"
    }
  }
}

function Invoke-RepairWorkspaceState {
  $workspaceRoot = Resolve-WorkspaceRootForRepair
  if (-not $workspaceRoot) {
    Write-Host "[infring install] repair skipped workspace cleanup (workspace root not detected)"
    return
  }
  $timestamp = Get-Date -Format "yyyyMMddTHHmmssZ"
  $archiveDir = Join-Path $workspaceRoot "local/workspace/archive/install-repair"
  New-Item -ItemType Directory -Force -Path $archiveDir | Out-Null

  $memoryPath = Join-Path $workspaceRoot "local/workspace/memory"
  if (Test-Path $memoryPath) {
    $memoryArchive = Join-Path $archiveDir "memory-$timestamp.zip"
    try {
      Compress-Archive -Path $memoryPath -DestinationPath $memoryArchive -Force
      Write-Host "[infring install] repair archived local/workspace/memory to $memoryArchive"
    } catch {
      Write-Host "[infring install] repair warning: failed to archive memory path ($memoryPath)"
    }
  }

  $statePath = Join-Path $workspaceRoot "local/state"
  if (Test-Path $statePath) {
    $stateArchive = Join-Path $archiveDir "state-$timestamp.zip"
    try {
      Compress-Archive -Path $statePath -DestinationPath $stateArchive -Force
      Write-Host "[infring install] repair archived local/state to $stateArchive"
    } catch {
      Write-Host "[infring install] repair warning: failed to archive state path ($statePath)"
    }
  }

  $cleanup = @("client/runtime/local", "client/tmp", "core/local/tmp", "local/state")
  foreach ($rel in $cleanup) {
    $abs = Join-Path $workspaceRoot $rel
    if (Test-Path $abs) {
      Remove-Item -Force -Recurse $abs
      Write-Host "[infring install] repair removed stale runtime path: $rel"
    }
  }
  New-Item -ItemType Directory -Force -Path (Join-Path $workspaceRoot "local/state") | Out-Null
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
if ($InstallRepair) {
  Write-Host "[infring install] repair mode enabled"
  Invoke-RepairInstallDir
  Invoke-RepairWorkspaceState
}
$arch = Resolve-Arch
$osFlags = Resolve-HostOsFlags
$HostIsWindows = [bool]$osFlags.IsWindows
$HostIsLinux = [bool]$osFlags.IsLinux
$HostIsMacOS = [bool]$osFlags.IsMacOS
$triple = if ($HostIsWindows) {
  "$arch-pc-windows-msvc"
} elseif ($HostIsLinux) {
  "$arch-unknown-linux-gnu"
} elseif ($HostIsMacOS) {
  "$arch-apple-darwin"
} else {
  throw "Unsupported OS for installer"
}
  $version = Resolve-Version
$resolvedVersionLabel = $version

Write-Host "[infring install] version: $version"
Write-Host "[infring install] platform: $triple"
Write-Host "[infring install] install dir: $InstallDir"

$opsBin = Join-Path $InstallDir "infring-ops.exe"
$pureBin = Join-Path $InstallDir "infring-pure-workspace.exe"
$infringdBin = Join-Path $InstallDir "infringd.exe"
$daemonBin = Join-Path $InstallDir "conduit_daemon.exe"
$preferredDaemonTriple = if ($HostIsLinux -and $arch -eq "x86_64") { "x86_64-unknown-linux-musl" } else { $triple }

if ($HostIsWindows) {
  # Required stems are only install-critical binaries.
  # Daemon binaries are optional at install time (installer can run in spine mode),
  # so they must not block compatible-tag selection on Windows.
  $requiredWindowsStems = @()
  if ($InstallPure) {
    $requiredWindowsStems += "infring-pure-workspace"
  } else {
    $requiredWindowsStems += "infring-ops"
  }
  Invoke-WindowsInstallerPreflight -VersionTag $version -Triple $triple -RequiredStems $requiredWindowsStems
  if ($RequestedVersion -eq "latest") {
    $compatibleWindows = Resolve-AssetCompatibleVersionForTriple $triple $requiredWindowsStems
    if ($compatibleWindows -and ($compatibleWindows -ne $version)) {
      Write-Host "[infring install] latest release $version is missing one or more required Windows prebuilts for $triple; using compatible release $compatibleWindows"
      $version = $compatibleWindows
      $resolvedVersionLabel = $compatibleWindows
      Invoke-WindowsInstallerPreflight -VersionTag $version -Triple $triple -RequiredStems $requiredWindowsStems
    } elseif (-not $compatibleWindows) {
      Write-Host "[infring install] no compatible Windows prebuilt release found for required stems; source fallback remains a backup path only."
    }
  }
}

if ($InstallPure) {
  if (($RequestedVersion -eq "latest") -and (-not $HostIsWindows)) {
    $compatiblePure = Resolve-AssetCompatibleVersionForTriple $triple @("infring-pure-workspace")
    if ($compatiblePure -and ($compatiblePure -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish pure prebuilt assets for $triple; using compatible release $compatiblePure"
      $version = $compatiblePure
      $resolvedVersionLabel = $compatiblePure
    }
  }
  $pureInstalled = $false
  if ($InstallTinyMax) {
    $pureInstalled = Install-Binary $version $triple "infring-pure-workspace-tiny-max" $pureBin
  }
  if (-not $pureInstalled) {
    $pureInstalled = Install-Binary $version $triple "infring-pure-workspace" $pureBin
  }
  if (-not $pureInstalled) {
    $failureHint = Format-BinaryInstallFailureHint -Stem "infring-pure-workspace" -Triple $triple -VersionTag $version
    throw "Failed to install pure workspace binary for $triple ($resolvedVersionLabel). No compatible prebuilt asset was found and source fallback did not complete. Diagnostic: $failureHint Install Rust toolchain + C++ build tools, then rerun the README Windows install command: $ReadmeWindowsInstallCommand"
  }
  if ($InstallTinyMax) {
    Write-Host "[infring install] tiny-max pure mode selected: Rust-only tiny profile installed"
  } else {
    Write-Host "[infring install] pure mode selected: Rust-only client installed"
  }
} else {
  if (($RequestedVersion -eq "latest") -and (-not $HostIsWindows)) {
    $compatibleOps = Resolve-AssetCompatibleVersionForTriple $triple @("infring-ops")
    if ($compatibleOps -and ($compatibleOps -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish core ops runtime prebuilt assets for $triple; using compatible release $compatibleOps"
      $version = $compatibleOps
      $resolvedVersionLabel = $compatibleOps
    }
  }
  if (-not (Install-Binary $version $triple "infring-ops" $opsBin)) {
    $failureHint = Format-BinaryInstallFailureHint -Stem "infring-ops" -Triple $triple -VersionTag $version
    throw "Failed to install core ops runtime for $triple ($resolvedVersionLabel). Prebuilt asset download failed and source fallback did not complete. Diagnostic: $failureHint Install Rust toolchain + C++ build tools, then rerun the README Windows install command: $ReadmeWindowsInstallCommand"
  }
}

$daemonMode = "spine"
if ($InstallTinyMax -and (Install-Binary $version $preferredDaemonTriple "infringd-tiny-max" $infringdBin)) {
  $daemonMode = "infringd"
  Write-Host "[infring install] using tiny-max daemon runtime"
} elseif (Install-Binary $version $preferredDaemonTriple "infringd" $infringdBin) {
  $daemonMode = "infringd"
  if ($preferredDaemonTriple -eq "x86_64-unknown-linux-musl") {
    Write-Host "[infring install] using static musl daemon runtime (embedded-minimal-core)"
  } else {
    Write-Host "[infring install] using daemon runtime"
  }
} elseif ($preferredDaemonTriple -ne $triple -and (Install-Binary $version $triple "infringd" $infringdBin)) {
  $daemonMode = "infringd"
  Write-Host "[infring install] using native daemon runtime fallback"
} elseif (Install-Binary $version $triple "conduit_daemon" $daemonBin) {
  $daemonMode = "conduit"
  Write-Host "[infring install] using conduit_daemon compatibility fallback"
} else {
  Write-Host "[infring install] no dedicated daemon binary found; falling back to spine mode via core ops runtime"
}

$wrapperPrelude = @'
@echo off
setlocal EnableExtensions EnableDelayedExpansion
set "_infring_root="
if defined INFRING_WORKSPACE_ROOT call :_check_candidate "%INFRING_WORKSPACE_ROOT%"
if not defined _infring_root call :_search_up "%CD%"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if defined _infring_root (
  set "INFRING_WORKSPACE_ROOT=%_infring_root%"
  cd /d "%_infring_root%" >nul 2>&1
)
goto :_dispatch

:_check_candidate
set "_candidate=%~1"
if "%_candidate%"=="" goto :eof
if exist "%_candidate%\core\layer0\ops\Cargo.toml" if exist "%_candidate%\client\runtime" set "_infring_root=%_candidate%"
goto :eof

:_search_up
set "_probe=%~1"
:_search_up_loop
if "!_probe!"=="" goto :eof
call :_check_candidate "!_probe!"
if defined _infring_root goto :eof
for %%I in ("!_probe!") do set "_parent=%%~dpI"
if not defined _parent goto :eof
if "!_parent:~-1!"=="\" set "_parent=!_parent:~0,-1!"
if /I "!_parent!"=="!_probe!" goto :eof
set "_probe=!_parent!"
goto :_search_up_loop
'@

$gatewayDispatchTemplate = @'
:_dispatch
if /I "%~1"=="gateway" (
  shift
  call :_gateway_dispatch %*
  set "_gateway_rc=!ERRORLEVEL!"
  exit /b !_gateway_rc!
)
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!

:_gateway_usage
echo Usage: infring gateway [start^|stop^|restart^|status^|attach^|subscribe^|tick^|diagnostics] [flags]
echo   default action is 'start'
echo   add --dashboard-open=0 to skip browser auto-open on start
exit /b 0

:_gateway_dispatch
set "_gateway_arg1=%~1"
set "_gateway_action="
set "_gateway_shift=0"
if "%_gateway_arg1%"=="" set "_gateway_action=start"
if /I "%_gateway_arg1%"=="start" set "_gateway_action=start" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="boot" set "_gateway_action=start" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="stop" set "_gateway_action=stop" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="restart" set "_gateway_action=restart" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="status" set "_gateway_action=status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="attach" set "_gateway_action=attach" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="subscribe" set "_gateway_action=subscribe" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="tick" set "_gateway_action=tick" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="diagnostics" set "_gateway_action=diagnostics" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="efficiency-status" set "_gateway_action=efficiency-status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="embedded-core-status" set "_gateway_action=embedded-core-status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="--help" goto :_gateway_usage
if /I "%_gateway_arg1%"=="-h" goto :_gateway_usage
if /I "%_gateway_arg1%"=="help" goto :_gateway_usage
if not defined _gateway_action set "_gateway_action=start"
if "!_gateway_shift!"=="1" shift

set "_gateway_tmp=%TEMP%\infring-gateway-%RANDOM%-%RANDOM%.log"
call "%~dp0infringd.cmd" "!_gateway_action!" %* > "!_gateway_tmp!" 2>&1
set "_gateway_status=!ERRORLEVEL!"
if not "!_gateway_status!"=="0" (
  if exist "!_gateway_tmp!" type "!_gateway_tmp!" 1>&2
  echo [infring gateway] !_gateway_action! failed 1>&2
  if exist "!_gateway_tmp!" del /q "!_gateway_tmp!" >nul 2>&1
  exit /b !_gateway_status!
)

set "_gateway_raw=0"
if /I "%INFRING_GATEWAY_RAW%"=="1" set "_gateway_raw=1"
if "!_gateway_raw!"=="1" if exist "!_gateway_tmp!" type "!_gateway_tmp!"

if /I "!_gateway_action!"=="start" (
  set "_dashboard_url=%INFRING_DASHBOARD_URL%"
  if "!_dashboard_url!"=="" set "_dashboard_url=http://127.0.0.1:4173/dashboard#chat"
  set "_dashboard_open=1"
  if /I "%INFRING_NO_BROWSER%"=="1" set "_dashboard_open=0"
  for %%A in (%*) do (
    if /I "%%~A"=="--dashboard-open=0" set "_dashboard_open=0"
    if /I "%%~A"=="--dashboard-open=1" set "_dashboard_open=1"
    if /I "%%~A"=="--no-browser" set "_dashboard_open=0"
  )
  if "!_dashboard_open!"=="1" start "" "!_dashboard_url!" >nul 2>&1
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] runtime started
  echo [infring gateway] dashboard: !_dashboard_url!
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="stop" (
  echo [infring gateway] runtime stopped
) else if /I "!_gateway_action!"=="status" (
  echo [infring gateway] runtime status received
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="restart" (
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] runtime restarted
) else (
  echo [infring gateway] action complete: !_gateway_action!
)
if exist "!_gateway_tmp!" del /q "!_gateway_tmp!" >nul 2>&1
exit /b 0
'@

$plainDispatchTemplate = @'
:_dispatch
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!
'@

$daemonCompatDispatchTemplate = @'
:_dispatch
set "_daemon_cmd=%~1"
if /I "%_daemon_cmd%"=="daemon-control" goto :_compat_dispatch
if /I "%_daemon_cmd%"=="dashboard-ui" goto :_compat_dispatch
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!

:_compat_dispatch
set "_ops_domain=%INFRING_OPS_DOMAIN%"
if not defined _ops_domain set "_ops_domain=infringctl"
if exist "%~dp0infring-ops.exe" (
  call "%~dp0infring-ops.exe" "!_ops_domain!" %*
  set "_cmd_rc=!ERRORLEVEL!"
  exit /b !_cmd_rc!
)
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!
'@

function Write-CmdWrapper {
  param(
    [string]$Path,
    [string]$Entry,
    [string]$EntryArgs,
    [switch]$Gateway
  )

  $dispatch = if ($Gateway) { $gatewayDispatchTemplate } else { $plainDispatchTemplate }
  $dispatch = $dispatch.Replace("__ENTRY__", $Entry)
  if ([string]::IsNullOrWhiteSpace($EntryArgs)) {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", "")
  } else {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", $EntryArgs)
  }

  $content = $wrapperPrelude + "`r`n" + $dispatch + "`r`n"
  Set-Content -Path $Path -Value $content
}

function Write-DaemonCmdWrapper {
  param(
    [string]$Path,
    [string]$Entry,
    [string]$EntryArgs
  )

  $dispatch = $daemonCompatDispatchTemplate.Replace("__ENTRY__", $Entry)
  if ([string]::IsNullOrWhiteSpace($EntryArgs)) {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", "")
  } else {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", $EntryArgs)
  }

  $content = $wrapperPrelude + "`r`n" + $dispatch + "`r`n"
  Set-Content -Path $Path -Value $content
}

function Resolve-WorkspaceRootForSmoke {
  return Resolve-WorkspaceRootForRepair
}

function Show-DashboardFailureLogs {
  param(
    [string]$WorkspaceRoot
  )

  $root = if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    Resolve-WorkspaceRootForSmoke
  } else {
    $WorkspaceRoot
  }
  if ([string]::IsNullOrWhiteSpace($root)) {
    return
  }
  $stateDir = Join-Path $root "local/state/ops/daemon_control"
  foreach ($name in @("dashboard_ui.log", "dashboard_watchdog.log")) {
    $path = Join-Path $stateDir $name
    if (-not (Test-Path $path)) { continue }
    Write-Host "[infring install] tail $path"
    Get-Content -Path $path -Tail 80 -ErrorAction SilentlyContinue
  }
}

function Test-DashboardHealthSmoke {
  param(
    [string]$InstallDir,
    [string]$DashboardHost = "127.0.0.1",
    [int]$Port = 4173
  )

  $workspaceRoot = Resolve-WorkspaceRootForSmoke
  $healthLog = Join-Path ([System.IO.Path]::GetTempPath()) ("infring-dashboard-health-" + [guid]::NewGuid().ToString("N") + ".log")

  $null = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "stop", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0") -TimeoutSec 20

  $startResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "start", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0", "--gateway-persist=0") -TimeoutSec 45 -LogPath $healthLog
  if (-not [bool]$startResult.Ok) {
    if ([bool]$startResult.TimedOut) {
      Write-Host "[infring install] smoke dashboard_health: failed (gateway start timeout)"
    } else {
      Write-Host "[infring install] smoke dashboard_health: failed (gateway start)"
    }
    if ([bool]$startResult.LogPath -and (Test-Path $startResult.LogPath)) {
      Get-Content -Path $startResult.LogPath -Tail 120 -ErrorAction SilentlyContinue
    }
    if ([bool]$startResult.ErrPath -and (Test-Path $startResult.ErrPath)) {
      Get-Content -Path $startResult.ErrPath -Tail 120 -ErrorAction SilentlyContinue
    }
    Show-DashboardFailureLogs -WorkspaceRoot $workspaceRoot
    return $false
  }

  $ready = $false
  for ($i = 0; $i -lt 45; $i++) {
    try {
      Invoke-WebRequest -Uri "http://$DashboardHost`:$Port/healthz" -UseBasicParsing -TimeoutSec 2 | Out-Null
      $ready = $true
      break
    } catch {}
    Start-Sleep -Seconds 1
  }

  $null = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "stop", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0") -TimeoutSec 20

  if (-not $ready) {
    Write-Host "[infring install] smoke dashboard_health: failed (healthz timeout)"
    if (Test-Path $healthLog) { Get-Content -Path $healthLog -Tail 120 -ErrorAction SilentlyContinue }
    Show-DashboardFailureLogs -WorkspaceRoot $workspaceRoot
    return $false
  }

  Write-Host "[infring install] smoke dashboard_health: ok"
  return $true
}

function Invoke-InfringCmdWithTimeout {
  param(
    [string]$InstallDir,
    [string[]]$Arguments,
    [int]$TimeoutSec = 25,
    [string]$LogPath
  )

  $cmdPath = Join-Path $InstallDir "infring.cmd"
  if (-not (Test-Path $cmdPath)) {
    return @{
      Ok = $false
      ExitCode = 1
      TimedOut = $false
      Error = "missing_infring_cmd"
      LogPath = $null
      ErrPath = $null
    }
  }

  if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = Join-Path ([System.IO.Path]::GetTempPath()) ("infring-install-smoke-" + [guid]::NewGuid().ToString("N") + ".log")
  }
  $errPath = "$LogPath.err"

  $quotedArgs = @()
  foreach ($arg in $Arguments) {
    $escaped = [string]$arg
    $escaped = $escaped.Replace('"', '""')
    $quotedArgs += "`"$escaped`""
  }
  $commandLine = "call `"$cmdPath`""
  if ($quotedArgs.Count -gt 0) {
    $commandLine = "$commandLine " + ($quotedArgs -join " ")
  }

  try {
    $proc = Start-Process -FilePath "cmd.exe" -ArgumentList @("/d", "/s", "/c", $commandLine) -PassThru -WindowStyle Hidden -RedirectStandardOutput $LogPath -RedirectStandardError $errPath
  } catch {
    return @{
      Ok = $false
      ExitCode = 1
      TimedOut = $false
      Error = $_.Exception.Message
      LogPath = $LogPath
      ErrPath = $errPath
    }
  }

  $finished = $proc.WaitForExit($TimeoutSec * 1000)
  if (-not $finished) {
    try { $proc.Kill() } catch {}
    return @{
      Ok = $false
      ExitCode = $null
      TimedOut = $true
      Error = "timeout_${TimeoutSec}s"
      LogPath = $LogPath
      ErrPath = $errPath
    }
  }

  return @{
    Ok = ($proc.ExitCode -eq 0)
    ExitCode = $proc.ExitCode
    TimedOut = $false
    Error = $null
    LogPath = $LogPath
    ErrPath = $errPath
  }
}

$powerShellShimTemplate = @'
param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$CommandArgs
)
$target = Join-Path $PSScriptRoot "__TARGET__"
if (-not (Test-Path $target)) {
  throw "Missing command wrapper: $target"
}
__DEPRECATION__
& $target @CommandArgs
exit $LASTEXITCODE
'@

function Write-PowerShellShim {
  param(
    [string]$Path,
    [string]$TargetCmd,
    [string]$DeprecationMessage
  )

  $content = $powerShellShimTemplate.Replace("__TARGET__", $TargetCmd)
  $deprecationLine = ""
  if (-not [string]::IsNullOrWhiteSpace($DeprecationMessage)) {
    $deprecationEscaped = $DeprecationMessage.Replace('"', '""')
    $deprecationLine = "Write-Warning `"$deprecationEscaped`""
  }
  $content = $content.Replace("__DEPRECATION__", $deprecationLine)
  Set-Content -Path $Path -Value $content
}

$infringCmd = Join-Path $InstallDir "infring.cmd"
$infringctlCmd = Join-Path $InstallDir "infringctl.cmd"
$infringdCmd = Join-Path $InstallDir "infringd.cmd"

if ($InstallPure) {
  if ($InstallTinyMax) {
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs '--tiny-max=1' -Gateway
  } else {
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs '' -Gateway
  }
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs 'conduit' -Gateway
} else {
  Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'infringctl' -Gateway
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'infringctl' -Gateway
}

if ($daemonMode -eq "infringd") {
  Write-DaemonCmdWrapper -Path $infringdCmd -Entry '"%~dp0infringd.exe"' -EntryArgs ''
} elseif ($daemonMode -eq "conduit") {
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0conduit_daemon.exe"' -EntryArgs ''
} else {
  if ($InstallPure) {
    throw "No daemon binary available for pure mode"
  }
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'spine'
}

$infringPs1 = Join-Path $InstallDir "infring.ps1"
$infringctlPs1 = Join-Path $InstallDir "infringctl.ps1"
$infringdPs1 = Join-Path $InstallDir "infringd.ps1"

Write-PowerShellShim -Path $infringPs1 -TargetCmd "infring.cmd"
Write-PowerShellShim -Path $infringctlPs1 -TargetCmd "infringctl.cmd"
Write-PowerShellShim -Path $infringdPs1 -TargetCmd "infringd.cmd"

if ($InstallPure) {
  Write-Host "[infring install] pure mode: skipping Infring client bundle"
} elseif ($InstallFull) {
  $clientDir = Join-Path $InstallDir "infring-client"
  if (Install-ClientBundle $version $triple $clientDir) {
    Write-Host "[infring install] full mode enabled: client runtime installed at $clientDir"
  } elseif (Install-ClientBundleFromSourceFallback $clientDir) {
    Write-Host "[infring install] full mode enabled: client runtime installed from source fallback at $clientDir"
  } else {
    throw "Full mode requested but no client runtime bundle is available for $triple ($version), and source fallback runtime copy was unavailable."
  }
} else {
  Write-Host "[infring install] lazy mode: skipping TS systems/eyes client bundle (use -Full to include)"
}

$machinePath = [Environment]::GetEnvironmentVariable("Path", "User")
$userPathResult = Ensure-WindowsPathContains $machinePath $InstallDir -PreferFront -RemoveEntries @($legacyInstallDir)
if ([bool]$userPathResult.Changed) {
  [Environment]::SetEnvironmentVariable("Path", [string]$userPathResult.Value, "User")
  if ([bool]$userPathResult.Added) {
    Write-Host "[infring install] added install dir to user PATH"
  } else {
    Write-Host "[infring install] normalized user PATH entries"
  }
}
$sessionPathResult = Ensure-WindowsPathContains $env:Path $InstallDir -PreferFront -RemoveEntries @($legacyInstallDir)
$env:Path = [string]$sessionPathResult.Value

$resolvedInfring = Get-Command infring -ErrorAction SilentlyContinue
if ($null -ne $resolvedInfring) {
  Write-Host "[infring install] shell command resolves to: $($resolvedInfring.Source)"
  $resolvedNorm = Normalize-WindowsPathEntry $resolvedInfring.Source
  $installNorm = Normalize-WindowsPathEntry $InstallDir
  if ($installNorm -and (-not $resolvedNorm.StartsWith($installNorm))) {
    Write-Host "[infring install] warning: current shell still prefers a non-canonical infring shim; use direct path fallback or start a new PowerShell session."
  }
} else {
  Write-Host "[infring install] warning: shell command resolution for 'infring' not ready in this session; use direct path fallback."
}

$gatewaySmokeOk = $false
$gatewaySmokeError = ""
$gatewaySmokeResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "status", "--auto-heal=0", "--dashboard-open=0") -TimeoutSec 25
if ([bool]$gatewaySmokeResult.Ok) {
  $gatewaySmokeOk = $true
} elseif ([bool]$gatewaySmokeResult.TimedOut) {
  $gatewaySmokeError = "timeout"
} elseif ($null -ne $gatewaySmokeResult.ExitCode) {
  $gatewaySmokeError = "exit_code_$($gatewaySmokeResult.ExitCode)"
} elseif (-not [string]::IsNullOrWhiteSpace([string]$gatewaySmokeResult.Error)) {
  $gatewaySmokeError = [string]$gatewaySmokeResult.Error
} else {
  $gatewaySmokeError = "unknown"
}
if ($gatewaySmokeOk) {
  Write-Host "[infring install] smoke gateway_status: ok"
} else {
  Write-Host "[infring install] smoke gateway_status: failed ($gatewaySmokeError)"
  if ([bool]$gatewaySmokeResult.LogPath -and (Test-Path $gatewaySmokeResult.LogPath)) {
    Get-Content -Path $gatewaySmokeResult.LogPath -Tail 80 -ErrorAction SilentlyContinue
  }
  if ([bool]$gatewaySmokeResult.ErrPath -and (Test-Path $gatewaySmokeResult.ErrPath)) {
    Get-Content -Path $gatewaySmokeResult.ErrPath -Tail 80 -ErrorAction SilentlyContinue
  }
}

$dashboardSmokeRequired = $InstallFull
if ($env:INFRING_INSTALL_STRICT_SMOKE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_STRICT_SMOKE.ToLower()) {
  $dashboardSmokeRequired = $true
}
if ($dashboardSmokeRequired) {
  $smokePort = 4400 + (Get-Random -Minimum 0 -Maximum 1000)
  if (-not (Test-DashboardHealthSmoke -InstallDir $InstallDir -DashboardHost "127.0.0.1" -Port $smokePort)) {
    throw "Full install failed dashboard health smoke."
  }
} else {
  Write-Host "[infring install] smoke dashboard_health: skipped (set INFRING_INSTALL_STRICT_SMOKE=1 or use -Full to enforce)"
}

Write-Host "[infring install] installed: infring, infringctl, infringd"
Write-Host "[infring install] run now (direct path): $InstallDir\\infring.cmd --help"
Write-Host "[infring install] quickstart now (direct path): $InstallDir\\infring.cmd gateway"
Write-Host "[infring install] run in this shell: infring --help"
Write-Host "[infring install] quickstart: infring gateway"
Write-Host "[infring install] stop: infring gateway stop"
Write-Host "[infring install] if command isn't found immediately, run: $InstallDir\\infring.cmd --help"
Write-Host "[infring install] if `Remove-Item` prints nothing, that's expected success behavior in PowerShell."
Write-Host "[infring install] README Windows install command: $ReadmeWindowsInstallCommand"

Invoke-SourceFallbackCleanup
