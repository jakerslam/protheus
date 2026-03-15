param(
  [switch]$Full,
  [switch]$Minimal,
  [switch]$Pure
)

$ErrorActionPreference = "Stop"

$RepoOwner = "protheuslabs"
$RepoName = "protheus"
$DefaultApi = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
$DefaultBase = "https://github.com/$RepoOwner/$RepoName/releases/download"

$InstallDir = if ($env:PROTHEUS_INSTALL_DIR) { $env:PROTHEUS_INSTALL_DIR } else { Join-Path $HOME ".protheus\bin" }
$RequestedVersion = if ($env:PROTHEUS_VERSION) { $env:PROTHEUS_VERSION } else { "latest" }
$ApiUrl = if ($env:PROTHEUS_RELEASE_API_URL) { $env:PROTHEUS_RELEASE_API_URL } else { $DefaultApi }
$BaseUrl = if ($env:PROTHEUS_RELEASE_BASE_URL) { $env:PROTHEUS_RELEASE_BASE_URL } else { $DefaultBase }
$InstallFull = $false
if ($env:PROTHEUS_INSTALL_FULL -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_FULL.ToLower()) {
  $InstallFull = $true
}
$InstallPure = $false
if ($env:PROTHEUS_INSTALL_PURE -and @("1", "true", "yes", "on") -contains $env:PROTHEUS_INSTALL_PURE.ToLower()) {
  $InstallPure = $true
}
if ($Full) { $InstallFull = $true }
if ($Minimal) { $InstallFull = $false }
if ($Pure) {
  $InstallPure = $true
  $InstallFull = $false
}

function Resolve-Arch {
  $archRaw = if ($env:PROCESSOR_ARCHITECTURE) { $env:PROCESSOR_ARCHITECTURE } else { [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() }
  switch ($archRaw.ToLower()) {
    "amd64" { "x86_64" }
    "arm64" { "aarch64" }
    default { throw "Unsupported architecture: $archRaw" }
  }
}

function Resolve-Version {
  if ($RequestedVersion -ne "latest") {
    if ($RequestedVersion.StartsWith("v")) { return $RequestedVersion }
    return "v$RequestedVersion"
  }

  $release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
  if (-not $release.tag_name) {
    throw "Failed to resolve latest release tag"
  }
  return $release.tag_name
}

function Download-Asset($Version, $Asset, $OutPath) {
  $url = "$BaseUrl/$Version/$Asset"
  try {
    Invoke-WebRequest -Uri $url -OutFile $OutPath -UseBasicParsing | Out-Null
    Write-Host "[protheus install] downloaded $Asset"
    return $true
  } catch {
    return $false
  }
}

function Install-Binary($Version, $Triple, $Stem, $OutPath) {
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

  return $false
}

function Install-ClientBundle($Version, $Triple, $OutDir) {
  New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null
  $archive = Join-Path $tmp.FullName "client-runtime.bundle"
  function Expand-ClientArchive($ArchivePath, $Destination) {
    if ($ArchivePath.EndsWith(".tar.zst")) {
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
        Write-Host "[protheus install] skipping .tar.zst bundle (zstd unavailable); falling back to .tar.gz assets"
        return $false
      }
    }
    if ($ArchivePath.EndsWith(".tar.gz")) {
      tar -xzf $ArchivePath -C $Destination
      return $true
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
      if (Expand-ClientArchive $archive $OutDir) {
        Write-Host "[protheus install] installed optional client runtime bundle"
        return $true
      }
    }
  }
  return $false
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$arch = Resolve-Arch
$triple = if ($IsWindows) {
  "$arch-pc-windows-msvc"
} elseif ($IsLinux) {
  "$arch-unknown-linux-gnu"
} elseif ($IsMacOS) {
  "$arch-apple-darwin"
} else {
  throw "Unsupported OS for installer"
}
$version = Resolve-Version

Write-Host "[protheus install] version: $version"
Write-Host "[protheus install] platform: $triple"
Write-Host "[protheus install] install dir: $InstallDir"

$opsBin = Join-Path $InstallDir "protheus-ops.exe"
$pureBin = Join-Path $InstallDir "protheus-pure-workspace.exe"
$protheusdBin = Join-Path $InstallDir "protheusd.exe"
$daemonBin = Join-Path $InstallDir "conduit_daemon.exe"
$preferredDaemonTriple = if ($IsLinux -and $arch -eq "x86_64") { "x86_64-unknown-linux-musl" } else { $triple }

if ($InstallPure) {
  if (-not (Install-Binary $version $triple "protheus-pure-workspace" $pureBin)) {
    throw "Failed to download protheus-pure-workspace for $triple ($version)"
  }
  Write-Host "[protheus install] pure mode selected: Rust-only client installed"
} elseif (-not (Install-Binary $version $triple "protheus-ops" $opsBin)) {
  throw "Failed to download protheus-ops for $triple ($version)"
}

$daemonMode = "spine"
if (Install-Binary $version $preferredDaemonTriple "protheusd" $protheusdBin) {
  $daemonMode = "protheusd"
  if ($preferredDaemonTriple -eq "x86_64-unknown-linux-musl") {
    Write-Host "[protheus install] using static musl protheusd (embedded-minimal-core)"
  } else {
    Write-Host "[protheus install] using protheusd"
  }
} elseif ($preferredDaemonTriple -ne $triple -and (Install-Binary $version $triple "protheusd" $protheusdBin)) {
  $daemonMode = "protheusd"
  Write-Host "[protheus install] using native protheusd fallback"
} elseif (Install-Binary $version $triple "conduit_daemon" $daemonBin) {
  $daemonMode = "conduit"
  Write-Host "[protheus install] using conduit_daemon compatibility fallback"
} else {
  Write-Host "[protheus install] no dedicated daemon binary found; falling back to protheus-ops spine mode"
}

$protheusCmd = Join-Path $InstallDir "protheus.cmd"
if ($InstallPure) {
  Set-Content -Path $protheusCmd -Value "@echo off`r`n`"%~dp0protheus-pure-workspace.exe`" %*"
} else {
  Set-Content -Path $protheusCmd -Value "@echo off`r`n`"%~dp0protheus-ops.exe`" protheusctl %*"
}

$protheusctlCmd = Join-Path $InstallDir "protheusctl.cmd"
if ($InstallPure) {
  Set-Content -Path $protheusctlCmd -Value "@echo off`r`n`"%~dp0protheus-pure-workspace.exe`" conduit %*"
} else {
  Set-Content -Path $protheusctlCmd -Value "@echo off`r`n`"%~dp0protheus-ops.exe`" protheusctl %*"
}

$protheusdCmd = Join-Path $InstallDir "protheusd.cmd"
if ($daemonMode -eq "protheusd") {
  Set-Content -Path $protheusdCmd -Value "@echo off`r`n`"%~dp0protheusd.exe`" %*"
} elseif ($daemonMode -eq "conduit") {
  Set-Content -Path $protheusdCmd -Value "@echo off`r`n`"%~dp0conduit_daemon.exe`" %*"
} else {
  if ($InstallPure) {
    throw "No daemon binary available for pure mode"
  }
  Set-Content -Path $protheusdCmd -Value "@echo off`r`n`"%~dp0protheus-ops.exe`" spine %*"
}

if ($InstallPure) {
  Write-Host "[protheus install] pure mode: skipping OpenClaw client bundle"
} elseif ($InstallFull) {
  $clientDir = Join-Path $InstallDir "protheus-client"
  if (Install-ClientBundle $version $triple $clientDir) {
    Write-Host "[protheus install] full mode enabled: client runtime installed at $clientDir"
  } else {
    Write-Host "[protheus install] full mode requested but no client runtime bundle was published for this release"
  }
} else {
  Write-Host "[protheus install] lazy mode: skipping TS systems/eyes client bundle (use -Full to include)"
}

$machinePath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($machinePath -notlike "*$InstallDir*") {
  [Environment]::SetEnvironmentVariable("Path", "$machinePath;$InstallDir", "User")
  Write-Host "[protheus install] added install dir to user PATH"
}

Write-Host "[protheus install] installed: protheus, protheusctl, protheusd"
Write-Host "[protheus install] open a new terminal and run: protheus --help"
