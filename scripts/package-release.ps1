# SPDX-License-Identifier: MPL-2.0
param(
  [Parameter(Mandatory=$true)][string]$TargetDirectory,
  [Parameter(Mandatory=$true)][string]$OutputDirectory,
  [string]$Version='1.0.0'
)
$ErrorActionPreference='Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$') {throw 'A stable numeric version is required'}
$project=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$target=[IO.Path]::GetFullPath($TargetDirectory)
$output=[IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $output) -and @(Get-ChildItem -LiteralPath $output -Force).Count -gt 0) {throw 'Use a new empty release output directory'}
New-Item -ItemType Directory -Force -Path $output | Out-Null
$installer=Join-Path $target "release\bundle\nsis\Glyphibe_${Version}_x64-setup.exe"
$desktop=Join-Path $target 'release\glyphibe-desktop.exe'
$cli=Join-Path $target 'release\glyphibe.exe'
foreach($file in @($installer,$desktop,$cli)){if(-not(Test-Path -LiteralPath $file -PathType Leaf)){throw 'Required executable is missing'}}
$reported=(& $cli --version | Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $reported -ne "glyphibe $Version"){throw 'CLI version does not match package version'}
Copy-Item -LiteralPath $installer -Destination (Join-Path $output (Split-Path -Leaf $installer))
Copy-Item -LiteralPath $cli -Destination (Join-Path $output "glyphibe-${Version}-windows-x64.exe")
foreach($kind in @('portable','cli')){
  $stage=Join-Path $output "stage-$kind"
  $legal=Join-Path $stage 'licenses'
  New-Item -ItemType Directory -Force -Path (Join-Path $legal 'third-party') | Out-Null
  Copy-Item -LiteralPath $cli -Destination (Join-Path $stage 'glyphibe.exe')
  if($kind -eq 'portable'){Copy-Item -LiteralPath $desktop -Destination (Join-Path $stage 'glyphibe-desktop.exe')}
  Copy-Item -LiteralPath (Join-Path $project 'LICENSE'),(Join-Path $project 'THIRD_PARTY_NOTICES') -Destination $legal
  Copy-Item -LiteralPath (Join-Path $project 'third-party\licenses') -Destination (Join-Path $legal 'third-party') -Recurse
  "Glyphibe $Version`nSource: https://github.com/styayur/glyphibe/tree/v$Version`nUnsigned Windows x64 build. Desktop requires WebView2. See licenses/THIRD_PARTY_NOTICES and the release notes for verification and limits.`nCLI: glyphibe --help`nMCP: glyphibe --repo PATH mcp serve" | Set-Content -LiteralPath (Join-Path $stage 'README.txt') -Encoding utf8
  Compress-Archive -Path (Join-Path $stage '*') -DestinationPath (Join-Path $output "Glyphibe-${Version}-windows-x64-${kind}.zip")
}
Copy-Item -LiteralPath (Join-Path $project 'docs\development\dependencies.spdx.json') -Destination (Join-Path $output 'dependencies.spdx.json')
$files=@(Get-ChildItem -LiteralPath $output -File | Sort-Object Name)
$records=@($files | ForEach-Object { [ordered]@{file=$_.Name;bytes=$_.Length;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();platform='Windows';architecture='x64';signed=$false} })
[ordered]@{version=$Version;source_commit=(git -C $project rev-parse HEAD);assets=$records;notes='Staging directories are local packaging inputs, not release assets. Upload only top-level files.'} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'asset-manifest.json') -Encoding utf8
Get-ChildItem -LiteralPath $output -File | Sort-Object Name | ForEach-Object { "$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())  $($_.Name)" } | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS') -Encoding ascii
$records | ConvertTo-Json -Depth 6
