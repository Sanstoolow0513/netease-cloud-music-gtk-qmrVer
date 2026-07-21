param(
    [Parameter(Mandatory = $true)][string]$InstallRoot,
    [Parameter(Mandatory = $true)][string]$DependencyPrefix,
    [string]$OutputDir,
    [string]$Version
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$installRoot = (Resolve-Path -LiteralPath $InstallRoot).Path
$dependencyPrefix = (Resolve-Path -LiteralPath $DependencyPrefix).Path

. (Join-Path $PSScriptRoot "env.ps1")
Initialize-Vs2022Environment
Initialize-NcmWindowsBuildEnvironment -DependencyPrefix $dependencyPrefix

if ($dependencyPrefix -match "\\(msys64|mingw64|ucrt64)\\") {
    throw "MinGW/MSYS2 dependency prefixes are not allowed: $dependencyPrefix"
}
if (-not $OutputDir) {
    $OutputDir = Join-Path $repositoryRoot "_windows\dist"
}
if (-not $Version) {
    $cargoToml = Get-Content -LiteralPath (Join-Path $repositoryRoot "Cargo.toml") -Raw
    $match = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Unable to read the application version from Cargo.toml"
    }
    $Version = $match.Groups[1].Value
}

function Copy-DirectoryContents {
    param([string]$Source, [string]$Destination)
    if (-not (Test-Path -LiteralPath $Source)) {
        return
    }
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

$packageName = "netease-cloud-music-gtk4-$Version-windows-x64"
$staging = Join-Path $OutputDir $packageName
$zipPath = Join-Path $OutputDir "$packageName.zip"

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
New-Item -ItemType Directory -Path $staging | Out-Null

Copy-DirectoryContents -Source $installRoot -Destination $staging

$installedExe = Join-Path $staging "bin\netease-cloud-music-gtk4.exe"
if (-not (Test-Path -LiteralPath $installedExe)) {
    throw "The installed application executable was not found: $installedExe"
}
Copy-Item -LiteralPath $installedExe -Destination $staging -Force
Remove-Item -LiteralPath (Join-Path $staging "bin") -Recurse -Force

Get-ChildItem -LiteralPath (Join-Path $dependencyPrefix "bin") -Filter "*.dll" | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $staging -Force
}

$pixbufQuery = Join-Path $dependencyPrefix "bin\gdk-pixbuf-query-loaders.exe"
if (Test-Path -LiteralPath $pixbufQuery) {
    Copy-Item -LiteralPath $pixbufQuery -Destination $staging -Force
}

Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "lib\gstreamer-1.0") `
    -Destination (Join-Path $staging "lib\gstreamer-1.0")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "lib\gio\modules") `
    -Destination (Join-Path $staging "lib\gio\modules")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "lib\gdk-pixbuf-2.0") `
    -Destination (Join-Path $staging "lib\gdk-pixbuf-2.0")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "libexec") `
    -Destination (Join-Path $staging "libexec")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "etc") `
    -Destination (Join-Path $staging "etc")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "share\icons") `
    -Destination (Join-Path $staging "share\icons")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "share\themes") `
    -Destination (Join-Path $staging "share\themes")
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "share\gstreamer-1.0") `
    -Destination (Join-Path $staging "share\gstreamer-1.0")

$schemaDir = Join-Path $staging "share\glib-2.0\schemas"
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "share\glib-2.0\schemas") `
    -Destination $schemaDir
$compileSchemas = Join-Path $dependencyPrefix "bin\glib-compile-schemas.exe"
& $compileSchemas --strict $schemaDir
if ($LASTEXITCODE -ne 0) {
    throw "glib-compile-schemas failed with exit code $LASTEXITCODE"
}

Copy-Item -LiteralPath (Join-Path $repositoryRoot "COPYING") -Destination $staging -Force
Copy-Item `
    -LiteralPath (Join-Path $PSScriptRoot "README-WINDOWS.txt") `
    -Destination $staging `
    -Force

$noticeDir = Join-Path $staging "THIRD_PARTY_NOTICES"
New-Item -ItemType Directory -Path $noticeDir -Force | Out-Null
Copy-DirectoryContents `
    -Source (Join-Path $dependencyPrefix "share\licenses") `
    -Destination $noticeDir
if (-not (Get-ChildItem -LiteralPath $noticeDir -Force)) {
    @"
This package includes GTK, Libadwaita, GStreamer, gettext, OpenSSL and their
transitive dependencies built by gvsbuild. Refer to each upstream project for
its complete license terms and corresponding source.
"@ | Set-Content -LiteralPath (Join-Path $noticeDir "README.txt") -Encoding utf8
}

@"
version=$Version
target=x86_64-pc-windows-msvc
dependency_stack=gvsbuild-msvc
git_commit=$(git -C $repositoryRoot rev-parse HEAD)
"@ | Set-Content -LiteralPath (Join-Path $staging "build-info.txt") -Encoding utf8

$dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
if (-not $dumpbin) {
    throw "dumpbin.exe was not found. Run packaging from a VS 2022 Developer PowerShell."
}
Get-ChildItem -LiteralPath $staging -Recurse -Include "*.exe", "*.dll" | ForEach-Object {
    $imports = & $dumpbin.Source /dependents $_.FullName 2>&1
    if ($imports -match "libgcc|libwinpthread|libstdc\+\+|msys-2\.0") {
        throw "MinGW/MSYS runtime import detected in $($_.FullName)"
    }
}

Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zipPath -CompressionLevel Optimal
Write-Output $zipPath
