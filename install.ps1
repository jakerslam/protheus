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

$InstallDir = if ($InstallDir) {
  $InstallDir
} elseif ($env:INFRING_INSTALL_DIR) {
  $env:INFRING_INSTALL_DIR
} elseif ($env:PROTHEUS_INSTALL_DIR) {
  $env:PROTHEUS_INSTALL_DIR
} else {
  Join-Path $HOME ".infring\bin"
}
$TmpDir = if ($TmpDir) {
  $TmpDir
} elseif ($env:INFRING_TMP_DIR) {
  $env:INFRING_TMP_DIR
} elseif ($env:PROTHEUS_TMP_DIR) {
  $env:PROTHEUS_TMP_DIR
} else {
  $null
}
$RequestedVersion = if ($env:INFRING_VERSION) { $env:INFRING_VERSION } elseif ($env:PROTHEUS_VERSION) { $env:PROTHEUS_VERSION } else { "latest" }
$ApiUrl = if ($env:INFRING_RELEASE_API_URL) { $env:INFRING_RELEASE_API_URL } elseif ($env:PROTHEUS_RELEASE_API_URL) { $env:PROTHEUS_RELEASE_API_URL } else { $DefaultApi }
$ReleasesApiUrl = if ($env:INFRING_RELEASES_API_URL) { $env:INFRING_RELEASES_API_URL } elseif ($env:PROTHEUS_RELEASES_API_URL) { $env:PROTHEUS_RELEASES_API_URL } else { $DefaultReleasesApi }
$LatestUrl = if ($env:INFRING_RELEASE_LATEST_URL) { $env:INFRING_RELEASE_LATEST_URL } elseif ($env:PROTHEUS_RELEASE_LATEST_URL) { $env:PROTHEUS_RELEASE_LATEST_URL } else { $DefaultLatestUrl }
$BaseUrl = if ($env:INFRING_RELEASE_BASE_URL) { $env:INFRING_RELEASE_BASE_URL } elseif ($env:PROTHEUS_RELEASE_BASE_URL) { $env:PROTHEUS_RELEASE_BASE_URL } else { $DefaultBase }
$InstallFull = $false
if ($env:INFRING_INSTALL_FULL -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_FULL.ToLower()) {
  $InstallFull = $true
} elseif ($env:PROTHEUS_INSTALL_FULL -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_FULL.ToLower()) {
  $InstallFull = $true
}
$InstallPure = $false
if ($env:INFRING_INSTALL_PURE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_PURE.ToLower()) {
  $InstallPure = $true
} elseif ($env:PROTHEUS_INSTALL_PURE -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_PURE.ToLower()) {
  $InstallPure = $true
}
$InstallTinyMax = $false
if ($env:INFRING_INSTALL_TINY_MAX -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_TINY_MAX.ToLower()) {
  $InstallTinyMax = $true
} elseif ($env:PROTHEUS_INSTALL_TINY_MAX -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_TINY_MAX.ToLower()) {
  $InstallTinyMax = $true
}
$InstallRepair = $false
if ($env:INFRING_INSTALL_REPAIR -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_REPAIR.ToLower()) {
  $InstallRepair = $true
} elseif ($env:PROTHEUS_INSTALL_REPAIR -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_REPAIR.ToLower()) {
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
    [bool]$global:IsWindows
  } else {
    $isWindowsRuntime
  }
  $isLinux = if (Get-Variable -Name IsLinux -Scope Global -ErrorAction SilentlyContinue) {
    [bool]$global:IsLinux
  } else {
    $isLinuxRuntime
  }
  $isMacOS = if (Get-Variable -Name IsMacOS -Scope Global -ErrorAction SilentlyContinue) {
    [bool]$global:IsMacOS
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

function Ensure-WindowsPathContains([string]$pathValue, [string]$entry) {
  $parts = @()
  if (-not [string]::IsNullOrWhiteSpace($pathValue)) {
    $parts = $pathValue.Split(";") |
      ForEach-Object { [string]$_ } |
      ForEach-Object { $_.Trim().Trim('"') } |
      Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  }

  $entryClean = [string]$entry
  $entryNorm = Normalize-WindowsPathEntry $entryClean
  $seen = @{}
  $deduped = New-Object System.Collections.Generic.List[string]
  $containsEntry = $false

  foreach ($part in $parts) {
    $norm = Normalize-WindowsPathEntry $part
    if ([string]::IsNullOrWhiteSpace($norm)) {
      continue
    }
    if ($norm -eq $entryNorm) {
      $containsEntry = $true
    }
    if (-not $seen.ContainsKey($norm)) {
      $deduped.Add($part)
      $seen[$norm] = $true
    }
  }

  if (-not $containsEntry) {
    $deduped.Add($entryClean)
  }

  $joined = ($deduped -join ";")
  return @{
    Value = $joined
    Added = (-not $containsEntry)
    Changed = ($joined -ne [string]$pathValue)
  }
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

  $fallback = if ($env:INFRING_FALLBACK_VERSION) { $env:INFRING_FALLBACK_VERSION } elseif ($env:PROTHEUS_FALLBACK_VERSION) { $env:PROTHEUS_FALLBACK_VERSION } else { $null }
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

function Get-BinaryAssetCandidates([string]$Triple, [string]$Stem) {
  return @(
    "$Stem-$Triple.exe",
    "$Stem-$Triple",
    "$Stem-$Triple.bin",
    "$Stem.exe",
    "$Stem"
  )
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
      "protheus-ops" { return "protheus-ops" }
      "protheusd" { return "protheusd" }
      "protheusd-tiny-max" { return "protheusd" }
      "conduit_daemon" { return "conduit_daemon" }
      "protheus-pure-workspace" { return "protheus-pure-workspace" }
      "protheus-pure-workspace-tiny-max" { return "protheus-pure-workspace" }
      default { return $null }
    }
  }

  function Ensure-CargoToolchainForSourceFallback {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
      return $true
    }
    if (-not $HostIsWindows) {
      return $false
    }
    $autoRustup = if ($env:INFRING_INSTALL_AUTO_RUSTUP) {
      @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_AUTO_RUSTUP.ToLower()
    } elseif ($env:PROTHEUS_INSTALL_AUTO_RUSTUP) {
      @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_AUTO_RUSTUP.ToLower()
    } else {
      $true
    }
    if (-not $autoRustup) {
      return $false
    }
    Write-Host "[infring install] prebuilt binary not available; attempting Rust toolchain bootstrap for source fallback"
    $rustupExe = Join-Path ([System.IO.Path]::GetTempPath()) "rustup-init.exe"
    try {
      Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupExe -UseBasicParsing | Out-Null
      $proc = Start-Process -FilePath $rustupExe -ArgumentList "-y --profile minimal --default-toolchain stable" -Wait -PassThru
      if ($proc.ExitCode -ne 0) {
        return $false
      }
      $cargoBin = Join-Path $HOME ".cargo\bin"
      if (Test-Path $cargoBin) {
        if (-not $env:Path.ToLower().Contains($cargoBin.ToLower())) {
          $env:Path = "$cargoBin;$env:Path"
        }
      }
      return [bool](Get-Command cargo -ErrorAction SilentlyContinue)
    } catch {
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

    if (Get-Command git -ErrorAction SilentlyContinue) {
      try {
        git clone --depth 1 --branch $VersionTag $repoUrl $script:SourceFallbackDir | Out-Null
        return $script:SourceFallbackDir
      } catch {
        try {
          git clone --depth 1 $repoUrl $script:SourceFallbackDir | Out-Null
          return $script:SourceFallbackDir
        } catch {
        }
      }
    }

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
    return $null
  }

  function Install-BinaryFromSourceFallback([string]$VersionTag, [string]$StemName, [string]$OutBinaryPath) {
    $binName = Resolve-SourceBinName $StemName
    if (-not $binName) { return $false }

    $repoDir = Prepare-SourceFallbackRepo $VersionTag
    if (-not $repoDir) { return $false }

    $manifest = Join-Path $repoDir "core/layer0/ops/Cargo.toml"
    try {
      cargo build --release --manifest-path $manifest --bin $binName | Out-Null
    } catch {
      return $false
    }

    $built = Join-Path $repoDir "target/release/$binName.exe"
    if (-not (Test-Path $built)) { return $false }
    Copy-Item -Force $built $OutBinaryPath
    Write-Host "[infring install] built $binName from source fallback"
    return $true
  }

  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null

  $raw = Join-Path $tmp.FullName "$Stem.exe"
  if (Download-Asset $Version "$Stem-$Triple.exe" $raw) {
    Move-Item -Force $raw $OutPath
    return $true
  }

  if (Download-Asset $Version "$Stem-$Triple" $raw) {
    Move-Item -Force $raw $OutPath
    return $true
  }

  if (Download-Asset $Version "$Stem-$Triple.bin" $raw) {
    Move-Item -Force $raw $OutPath
    return $true
  }

  if (Download-Asset $Version "$Stem.exe" $raw) {
    Move-Item -Force $raw $OutPath
    return $true
  }

  if (Download-Asset $Version "$Stem" $raw) {
    Move-Item -Force $raw $OutPath
    return $true
  }

  return (Install-BinaryFromSourceFallback $Version $Stem $OutPath)
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
    "protheus-client-runtime-$Triple.tar.zst",
    "protheus-client-runtime.tar.zst",
    "protheus-client-$Triple.tar.zst",
    "protheus-client.tar.zst",
    "protheus-client-runtime-$Triple.tar.gz",
    "protheus-client-runtime.tar.gz",
    "protheus-client-$Triple.tar.gz",
    "protheus-client.tar.gz"
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

function Resolve-WorkspaceRootForRepair {
  $candidates = @(
    $env:INFRING_WORKSPACE_ROOT,
    $env:PROTHEUS_WORKSPACE_ROOT,
    (Get-Location).Path,
    (Join-Path $HOME ".infring/workspace")
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
    "protheus.cmd", "protheusctl.cmd", "protheusd.cmd",
    "protheus-ops.exe", "protheus-pure-workspace.exe",
    "protheusd.exe", "conduit_daemon.exe", "protheus-client"
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
$requestedVersion = $version

Write-Host "[infring install] version: $version"
Write-Host "[infring install] platform: $triple"
Write-Host "[infring install] install dir: $InstallDir"

$opsBin = Join-Path $InstallDir "protheus-ops.exe"
$pureBin = Join-Path $InstallDir "protheus-pure-workspace.exe"
$protheusdBin = Join-Path $InstallDir "protheusd.exe"
$daemonBin = Join-Path $InstallDir "conduit_daemon.exe"
$preferredDaemonTriple = if ($HostIsLinux -and $arch -eq "x86_64") { "x86_64-unknown-linux-musl" } else { $triple }

if ($InstallPure) {
  if ($RequestedVersion -eq "latest") {
    $compatiblePure = Resolve-AssetCompatibleVersionForTriple $triple @("protheus-pure-workspace")
    if ($compatiblePure -and ($compatiblePure -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish pure prebuilt assets for $triple; using compatible release $compatiblePure"
      $version = $compatiblePure
    }
  }
  $pureInstalled = $false
  if ($InstallTinyMax) {
    $pureInstalled = Install-Binary $version $triple "protheus-pure-workspace-tiny-max" $pureBin
  }
  if (-not $pureInstalled) {
    $pureInstalled = Install-Binary $version $triple "protheus-pure-workspace" $pureBin
  }
  if (-not $pureInstalled) {
    throw "Failed to install pure workspace binary for $triple ($requestedVersion). No compatible prebuilt asset was found and source fallback did not complete. Install Rust toolchain + C++ build tools, then retry with -Repair -Full."
  }
  if ($InstallTinyMax) {
    Write-Host "[infring install] tiny-max pure mode selected: Rust-only tiny profile installed"
  } else {
    Write-Host "[infring install] pure mode selected: Rust-only client installed"
  }
} else {
  if ($RequestedVersion -eq "latest") {
    $compatibleOps = Resolve-AssetCompatibleVersionForTriple $triple @("protheus-ops")
    if ($compatibleOps -and ($compatibleOps -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish core ops runtime prebuilt assets for $triple; using compatible release $compatibleOps"
      $version = $compatibleOps
    }
  }
  if (-not (Install-Binary $version $triple "protheus-ops" $opsBin)) {
    throw "Failed to install core ops runtime for $triple ($requestedVersion). Prebuilt asset download failed and source fallback did not complete. Install Rust toolchain + C++ build tools, then retry with -Repair -Full."
  }
}

$daemonMode = "spine"
if ($InstallTinyMax -and (Install-Binary $version $preferredDaemonTriple "protheusd-tiny-max" $protheusdBin)) {
  $daemonMode = "protheusd"
  Write-Host "[infring install] using tiny-max daemon runtime"
} elseif (Install-Binary $version $preferredDaemonTriple "protheusd" $protheusdBin) {
  $daemonMode = "protheusd"
  if ($preferredDaemonTriple -eq "x86_64-unknown-linux-musl") {
    Write-Host "[infring install] using static musl daemon runtime (embedded-minimal-core)"
  } else {
    Write-Host "[infring install] using daemon runtime"
  }
} elseif ($preferredDaemonTriple -ne $triple -and (Install-Binary $version $triple "protheusd" $protheusdBin)) {
  $daemonMode = "protheusd"
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
if not defined _infring_root if defined PROTHEUS_WORKSPACE_ROOT call :_check_candidate "%PROTHEUS_WORKSPACE_ROOT%"
if not defined _infring_root call :_search_up "%CD%"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if defined _infring_root (
  set "INFRING_WORKSPACE_ROOT=%_infring_root%"
  set "PROTHEUS_WORKSPACE_ROOT=%_infring_root%"
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
if /I "%PROTHEUS_GATEWAY_RAW%"=="1" set "_gateway_raw=1"
if "!_gateway_raw!"=="1" if exist "!_gateway_tmp!" type "!_gateway_tmp!"

if /I "!_gateway_action!"=="start" (
  set "_dashboard_url=%INFRING_DASHBOARD_URL%"
  if "!_dashboard_url!"=="" set "_dashboard_url=http://127.0.0.1:4173/dashboard#chat"
  set "_dashboard_open=1"
  if /I "%INFRING_NO_BROWSER%"=="1" set "_dashboard_open=0"
  if /I "%PROTHEUS_NO_BROWSER%"=="1" set "_dashboard_open=0"
  for %%A in (%*) do (
    if /I "%%~A"=="--dashboard-open=0" set "_dashboard_open=0"
    if /I "%%~A"=="--dashboard-open=1" set "_dashboard_open=1"
    if /I "%%~A"=="--no-browser" set "_dashboard_open=0"
  )
  if "!_dashboard_open!"=="1" start "" "!_dashboard_url!" >nul 2>&1
  echo [infring gateway] runtime started
  echo [infring gateway] dashboard: !_dashboard_url!
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="stop" (
  echo [infring gateway] runtime stopped
) else if /I "!_gateway_action!"=="status" (
  echo [infring gateway] runtime status received
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="restart" (
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

$powerShellShimTemplate = @'
param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Args
)
$target = Join-Path $PSScriptRoot "__TARGET__"
if (-not (Test-Path $target)) {
  throw "Missing command wrapper: $target"
}
__DEPRECATION__
& $target @Args
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
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0protheus-pure-workspace.exe"' -EntryArgs '--tiny-max=1' -Gateway
  } else {
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0protheus-pure-workspace.exe"' -EntryArgs '' -Gateway
  }
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0protheus-pure-workspace.exe"' -EntryArgs 'conduit' -Gateway
} else {
  Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0protheus-ops.exe"' -EntryArgs 'infringctl' -Gateway
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0protheus-ops.exe"' -EntryArgs 'infringctl' -Gateway
}

if ($daemonMode -eq "protheusd") {
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0protheusd.exe"' -EntryArgs ''
} elseif ($daemonMode -eq "conduit") {
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0conduit_daemon.exe"' -EntryArgs ''
} else {
  if ($InstallPure) {
    throw "No daemon binary available for pure mode"
  }
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0protheus-ops.exe"' -EntryArgs 'spine'
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
  $clientDir = Join-Path $InstallDir "protheus-client"
  if (Install-ClientBundle $version $triple $clientDir) {
    Write-Host "[infring install] full mode enabled: client runtime installed at $clientDir"
  } else {
    Write-Host "[infring install] full mode requested but no client runtime bundle was published for this release"
  }
} else {
  Write-Host "[infring install] lazy mode: skipping TS systems/eyes client bundle (use -Full to include)"
}

$machinePath = [Environment]::GetEnvironmentVariable("Path", "User")
$userPathResult = Ensure-WindowsPathContains $machinePath $InstallDir
if ([bool]$userPathResult.Changed) {
  [Environment]::SetEnvironmentVariable("Path", [string]$userPathResult.Value, "User")
  if ([bool]$userPathResult.Added) {
    Write-Host "[infring install] added install dir to user PATH"
  } else {
    Write-Host "[infring install] normalized user PATH entries"
  }
}
$sessionPathResult = Ensure-WindowsPathContains $env:Path $InstallDir
$env:Path = [string]$sessionPathResult.Value

$resolvedInfring = Get-Command infring -ErrorAction SilentlyContinue
if ($null -ne $resolvedInfring) {
  Write-Host "[infring install] shell command resolves to: $($resolvedInfring.Source)"
} else {
  Write-Host "[infring install] warning: shell command resolution for 'infring' not ready in this session; use direct path fallback."
}

$gatewaySmokeOk = $false
$gatewaySmokeError = ""
try {
  & "$InstallDir\\infring.cmd" gateway status --auto-heal=0 --dashboard-open=0 | Out-Null
  if ($LASTEXITCODE -eq 0) {
    $gatewaySmokeOk = $true
  } else {
    $gatewaySmokeError = "exit_code_$LASTEXITCODE"
  }
} catch {
  $gatewaySmokeError = $_.Exception.Message
}
if ($gatewaySmokeOk) {
  Write-Host "[infring install] smoke gateway_status: ok"
} else {
  Write-Host "[infring install] smoke gateway_status: failed ($gatewaySmokeError)"
}

Write-Host "[infring install] installed: infring, infringctl, infringd"
Write-Host "[infring install] run now (direct path): $InstallDir\\infring.cmd --help"
Write-Host "[infring install] quickstart now (direct path): $InstallDir\\infring.cmd gateway"
Write-Host "[infring install] run in this shell: infring --help"
Write-Host "[infring install] quickstart: infring gateway"
Write-Host "[infring install] stop: infring gateway stop"
Write-Host "[infring install] if command isn't found immediately, run: $InstallDir\\infring.cmd --help"
Write-Host "[infring install] if `Remove-Item` prints nothing, that's expected success behavior in PowerShell."
Write-Host "[infring install] if script execution is restricted, relaunch PowerShell with process-only bypass: Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force"

if ($script:SourceFallbackTmp -and (Test-Path $script:SourceFallbackTmp.FullName)) {
  Remove-Item -Force -Recurse $script:SourceFallbackTmp.FullName
}
