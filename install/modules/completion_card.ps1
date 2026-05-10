# SPDX-License-Identifier: Apache-2.0
# Layer ownership: installer/modules

function Write-InfringInstallCompletionCard {
  param(
    [Parameter(Mandatory = $true)][string] $Version,
    [Parameter(Mandatory = $true)][string] $Location,
    [string] $Command = 'infring --help'
  )
  Write-Host 'Setting up InfRing...'
  Write-Host ''
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
