# SPDX-License-Identifier: Apache-2.0
# Layer ownership: installer/modules

function Get-InfringWrapperSpecValue {
  param(
    [Parameter(Mandatory = $true)][object] $Spec,
    [Parameter(Mandatory = $true)][string] $Name
  )

  if ($Spec -is [hashtable] -and $Spec.ContainsKey($Name)) {
    return [string]$Spec[$Name]
  }

  $property = $Spec.PSObject.Properties[$Name]
  if ($null -ne $property) {
    return [string]$property.Value
  }

  return ""
}

function Write-InfringWindowsWrappers {
  param(
    [Parameter(Mandatory = $true)][string] $BinDir,
    [string] $TargetCommand = "",
    [object[]] $WrapperSpecs = @()
  )
  New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

  if ($WrapperSpecs -and $WrapperSpecs.Count -gt 0) {
    $written = @()
    foreach ($spec in $WrapperSpecs) {
      $cmdName = Get-InfringWrapperSpecValue -Spec $spec -Name "cmd"
      $ps1Name = Get-InfringWrapperSpecValue -Spec $spec -Name "ps1"
      $cmdBody = Get-InfringWrapperSpecValue -Spec $spec -Name "cmd_body"
      $ps1Body = Get-InfringWrapperSpecValue -Spec $spec -Name "ps1_body"

      if ([string]::IsNullOrWhiteSpace($cmdName) -or [string]::IsNullOrWhiteSpace($ps1Name)) {
        throw "windows_wrapper_spec_missing_name"
      }
      if ([string]::IsNullOrWhiteSpace($cmdBody) -or [string]::IsNullOrWhiteSpace($ps1Body)) {
        throw "windows_wrapper_spec_missing_body"
      }

      $cmdPath = Join-Path $BinDir $cmdName
      $ps1Path = Join-Path $BinDir $ps1Name
      Set-Content -LiteralPath $cmdPath -Value $cmdBody -Encoding ASCII
      Set-Content -LiteralPath $ps1Path -Value $ps1Body -Encoding UTF8
      $written += $cmdPath
      $written += $ps1Path
    }

    return @{ mode = "wrapper_specs"; wrappers = $written }
  }

  if ([string]::IsNullOrWhiteSpace($TargetCommand)) {
    throw "windows_wrapper_target_command_required"
  }

  $cmdPath = Join-Path $BinDir 'infring.cmd'
  $ps1Path = Join-Path $BinDir 'infring.ps1'
  $cmdBody = "@echo off`r`n`"$TargetCommand`" %*`r`n"
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
