# SPDX-License-Identifier: Apache-2.0
# Layer ownership: installer/modules

function Write-InfringWindowsWrappers {
  param(
    [Parameter(Mandatory = $true)][string] $BinDir,
    [Parameter(Mandatory = $true)][string] $TargetCommand
  )
  New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
  $cmdPath = Join-Path $BinDir 'infring.cmd'
  $ps1Path = Join-Path $BinDir 'infring.ps1'
  $cmdBody = "@echo off`r`n\"$TargetCommand\" %*`r`n"
  $ps1Body = @"
`$target = '$cmdPath'
if (-not (Test-Path -LiteralPath `$target)) {
  throw "Missing command wrapper: `$target"
}
& `$target @args
exit `$LASTEXITCODE
"@
  Set-Content -LiteralPath $cmdPath -Value $cmdBody -Encoding ASCII
  Set-Content -LiteralPath $ps1Path -Value $ps1Body -Encoding UTF8
  return @{ cmd = $cmdPath; ps1 = $ps1Path }
}

function Test-InfringWindowsWrappers {
  param([Parameter(Mandatory = $true)][string] $BinDir)
  $cmdPath = Join-Path $BinDir 'infring.cmd'
  $ps1Path = Join-Path $BinDir 'infring.ps1'
  return [ordered]@{
    ok = ((Test-Path -LiteralPath $cmdPath) -and (Test-Path -LiteralPath $ps1Path))
    cmd = $cmdPath
    ps1 = $ps1Path
    cmd_exists = (Test-Path -LiteralPath $cmdPath)
    ps1_exists = (Test-Path -LiteralPath $ps1Path)
  }
}
