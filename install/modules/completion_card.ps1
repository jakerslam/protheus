# SPDX-License-Identifier: Apache-2.0
# Layer ownership: installer/modules

function Write-InfringInstallCompletionCard {
  param(
    [Parameter(Mandatory = $true)][string] $Version,
    [Parameter(Mandatory = $true)][string] $Location,
    [string] $Command = 'infring --help',
    [bool] $RuntimeInstalled = $true,
    [string] $RuntimeMode = '',
    [string] $BootstrapOnlyReason = ''
  )
  $runtimeLabel = if ([string]::IsNullOrWhiteSpace($RuntimeMode)) {
    if ($RuntimeInstalled) { 'installed' } else { 'bootstrap_only_profile' }
  } else {
    $RuntimeMode
  }
  Write-Host 'Setting up InfRing...'
  Write-Host ''
  if (-not $RuntimeInstalled) {
    Write-Host 'BOOTSTRAP INSTALLED: InfRing runtime pending.' -ForegroundColor DarkYellow
    Write-Host ''
    Write-Host '  Version: ' -NoNewline
    Write-Host $Version -ForegroundColor DarkYellow
    Write-Host "  Location: $Location"
    Write-Host '  Runtime: ' -NoNewline
    Write-Host "$runtimeLabel (runtime binaries unavailable)" -ForegroundColor DarkYellow
    if (-not [string]::IsNullOrWhiteSpace($BootstrapOnlyReason)) {
      Write-Host "  Reason: $BootstrapOnlyReason"
    }
    Write-Host ''
    Write-Host '  Next: Run ' -NoNewline
    Write-Host 'infring recover' -ForegroundColor DarkYellow -NoNewline
    Write-Host ' after installing Windows Build Tools or after Windows runtime assets are available.'
    Write-Host ''
    Write-Host 'Installation incomplete: runtime pending.' -ForegroundColor DarkYellow
    return
  }
  Write-Host '✔ InfRing successfully installed!' -ForegroundColor Green
  Write-Host ''
  Write-Host '  Version: ' -NoNewline
  Write-Host $Version -ForegroundColor DarkYellow
  Write-Host "  Location: $Location"
  Write-Host ''
  Write-Host '  Next: Run ' -NoNewline
  Write-Host $Command -ForegroundColor DarkYellow -NoNewline
  Write-Host ' to get started.'
  Write-Host ''
  Write-Host 'Installation complete!' -ForegroundColor Green
}
