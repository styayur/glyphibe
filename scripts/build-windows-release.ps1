# SPDX-License-Identifier: MPL-2.0
param([Parameter(Mandatory=$true)][string]$TargetDirectory)
$ErrorActionPreference='Stop'
$project=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$target=[IO.Path]::GetFullPath($TargetDirectory)
$profileRoot=[IO.Path]::GetFullPath($env:USERPROFILE)
if(-not $env:CARGO_HOME){throw 'Use an isolated CARGO_HOME outside user, workspace and target prefixes for native C diagnostic paths'}
$registryRoot=[IO.Path]::GetFullPath($env:CARGO_HOME)
foreach($privateRoot in @($profileRoot,$project,$target)){
  if($registryRoot -eq $privateRoot -or $registryRoot.StartsWith($privateRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'The native C source cache must be outside the private path prefixes'}
}
$sourceCommit=(& git -C $project rev-parse HEAD | Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[a-f0-9]{40,64}$'){throw 'Reviewed source identity is unavailable'}
if((& git -C $project status --porcelain | Out-String).Trim()){throw 'Release builds require committed source'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if(-not(Test-Path -LiteralPath $vswhere)){throw 'Visual Studio build provenance is unavailable'}
$instances=@(& $vswhere -all -products '*' -format json | Out-String | ConvertFrom-Json)
if($instances.Count -eq 0){throw 'A stable Microsoft build-tool installation is required'}
$products=@()
foreach($instance in $instances){
  if($instance.isPrerelease -ne $false -or $instance.channelId -notmatch '^VisualStudio\.\d+\.Release$'){throw 'Official candidates require a Release channel; Insiders and prerelease tools are excluded'}
  $toolsetFile=Join-Path $instance.installationPath 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt'
  $products+=[ordered]@{displayName=$instance.displayName;productId=$instance.productId;installationVersion=$instance.installationVersion;channelId=$instance.channelId;isPrerelease=$instance.isPrerelease;installed_default_msvc_toolset=$(if(Test-Path -LiteralPath $toolsetFile){(Get-Content -LiteralPath $toolsetFile -Raw).Trim()}else{$null})}
}
if($env:RUSTFLAGS -and -not $env:CARGO_ENCODED_RUSTFLAGS){throw 'Use CARGO_ENCODED_RUSTFLAGS for additional flags so path arguments retain their boundaries'}
$previousTarget=$env:CARGO_TARGET_DIR
$previousFlags=$env:CARGO_ENCODED_RUSTFLAGS
$mappings=@(@($profileRoot,'/glyphibe-build/home'),@($project,'/glyphibe/source'),@($target,'/glyphibe-build/target'))
$flags=@()
if($previousFlags){$flags+= $previousFlags.Split([char]31)}
foreach($mapping in $mappings){
  $flags+="--remap-path-prefix=$($mapping[0])=$($mapping[1])"
  $flags+="--remap-path-prefix=$($mapping[0].Replace('\','/'))=$($mapping[1])"
}
$env:CARGO_ENCODED_RUSTFLAGS=$flags -join [char]31
$env:CARGO_TARGET_DIR=$target
Push-Location $project
try {
  & python scripts/check-version.py
  if($LASTEXITCODE -ne 0){throw 'Source versions differ'}
  $version=(Get-Content -LiteralPath (Join-Path $project 'apps/desktop/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
  if($version -notmatch '^\d+\.\d+\.\d+$'){throw 'A stable numeric source version is required'}
  & python scripts/audit-native-notices.py
  if($LASTEXITCODE -ne 0){throw 'Native notice checks failed'}
  & cargo build --locked --release -p glyphibe-cli
  if($LASTEXITCODE -ne 0){throw 'CLI release build failed'}
  Push-Location (Join-Path $project 'apps/desktop')
  try { & node node_modules/@tauri-apps/cli/tauri.js build --ci --bundles nsis -- --locked }
  finally { Pop-Location }
  if($LASTEXITCODE -ne 0){throw 'Desktop release build failed'}
  & python scripts/audit-native-notices.py --verify-build-host --target-directory $target
  if($LASTEXITCODE -ne 0){throw 'Actual native packaging components differ from the reviewed evidence'}
  function Get-ValidatedBuildRecord([string]$path) {
    $bytes=[IO.File]::ReadAllBytes($path)
    $utf8=[Text.Encoding]::UTF8.GetString($bytes)
    $utf16=[Text.Encoding]::Unicode.GetString($bytes)
    foreach($prefix in @($profileRoot,$project,$target)){
      foreach($needle in @($prefix,$prefix.Replace('\','/'))){
        if($utf8.Contains($needle,[StringComparison]::OrdinalIgnoreCase) -or $utf16.Contains($needle,[StringComparison]::OrdinalIgnoreCase)){throw 'A release payload still contains a private build path'}
      }
    }
    return [ordered]@{file=(Split-Path -Leaf $path);bytes=$bytes.Length;sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant();signature_status=(Get-AuthenticodeSignature -LiteralPath $path).Status.ToString();private_build_path_scan='passed'}
  }
  $payloads=@('glyphibe.exe','glyphibe-desktop.exe') | ForEach-Object { Get-ValidatedBuildRecord (Join-Path $target "release/$_") }
  $installer=Get-ValidatedBuildRecord (Join-Path $target "release/bundle/nsis/Glyphibe_${version}_x64-setup.exe")
  $report=[ordered]@{source_commit=$sourceCommit;built_at=[DateTimeOffset]::UtcNow.ToString('o');provider=$(if($env:GITHUB_ACTIONS -eq 'true'){'GitHub-hosted Actions'}else{'Local build'});run_id=$env:GITHUB_RUN_ID;runner_image=$env:ImageOS;runner_image_version=$env:ImageVersion;rust=(& rustc -vV | Out-String).Trim();cargo=(& cargo --version | Out-String).Trim();node=(& node --version | Out-String).Trim();visual_studio_products=$products;path_remapping=@('/glyphibe-build/home','/glyphibe/source','/glyphibe-build/target');payloads=@($payloads);installer=$installer;scope='Build provenance and private-path checks, not a reproducible-build or personal license entitlement certification.'}
  $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $target 'glyphibe-build-provenance.json') -Encoding utf8
}
finally { Pop-Location; $env:CARGO_TARGET_DIR=$previousTarget; $env:CARGO_ENCODED_RUSTFLAGS=$previousFlags }
