# Windows 本地开发：构建 → 同步到带 DLL 的便携包目录 → 启动应用。
# 用法（仓库根目录）:
#   make dev
#   .\build-aux\windows\dev.ps1
#   .\build-aux\windows\dev.ps1 -BuildType release
#   .\build-aux\windows\dev.ps1 -NoStart
#   .\build-aux\windows\dev.ps1 -Repackage   # 强制完整重打包（首次或依赖变更时）
param(
    [string]$DependencyPrefix,
    [ValidateSet("debug", "release")][string]$BuildType = "debug",
    [switch]$NoStart,
    [switch]$Repackage
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

function Resolve-DependencyPrefix {
    param([string]$Explicit)
    if ($Explicit) {
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    foreach ($candidate in @(
            "C:\ncm-gtk\gtk\x64\release"
            (Join-Path $repositoryRoot "_windows\gvsbuild\gtk\x64\release")
        )) {
        if (Test-Path -LiteralPath (Join-Path $candidate "bin\gtk-4.1.dll")) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
        if (Test-Path -LiteralPath (Join-Path $candidate "bin")) {
            $gtkDll = Get-ChildItem -LiteralPath (Join-Path $candidate "bin") -Filter "gtk-4*.dll" -ErrorAction SilentlyContinue |
                Select-Object -First 1
            if ($gtkDll) {
                return (Resolve-Path -LiteralPath $candidate).Path
            }
        }
    }
    throw @"
Windows dependency prefix not found.
Run bootstrap first:
  .\build-aux\windows\bootstrap.ps1
Then retry make dev / this script.
"@
}

function Get-AppVersion {
    $cargoToml = Get-Content -LiteralPath (Join-Path $repositoryRoot "Cargo.toml") -Raw
    $match = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Unable to read the application version from Cargo.toml"
    }
    return $match.Groups[1].Value
}

function Stop-AppIfRunning {
    $procs = Get-Process -Name "netease-cloud-music-gtk4" -ErrorAction SilentlyContinue
    if ($procs) {
        Write-Host "Stopping running netease-cloud-music-gtk4..."
        $procs | Stop-Process -Force
        Start-Sleep -Milliseconds 800
    }
}

function Sync-InstallToDist {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)][string]$DistDir,
        [Parameter(Mandatory = $true)][string]$Prefix
    )

    $exeSrc = Join-Path $InstallRoot "bin\netease-cloud-music-gtk4.exe"
    if (-not (Test-Path -LiteralPath $exeSrc)) {
        throw "Installed executable not found: $exeSrc"
    }
    Copy-Item -LiteralPath $exeSrc -Destination (Join-Path $DistDir "netease-cloud-music-gtk4.exe") -Force

    $gresourceSrc = Join-Path $InstallRoot "share\netease-cloud-music-gtk4\netease-cloud-music-gtk4.gresource"
    $gresourceDstDir = Join-Path $DistDir "share\netease-cloud-music-gtk4"
    New-Item -ItemType Directory -Path $gresourceDstDir -Force | Out-Null
    Copy-Item -LiteralPath $gresourceSrc -Destination (Join-Path $gresourceDstDir "netease-cloud-music-gtk4.gresource") -Force

    $localeSrc = Join-Path $InstallRoot "share\locale"
    $localeDst = Join-Path $DistDir "share\locale"
    if (Test-Path -LiteralPath $localeSrc) {
        New-Item -ItemType Directory -Path $localeDst -Force | Out-Null
        Copy-Item -Path (Join-Path $localeSrc "*") -Destination $localeDst -Recurse -Force
    }

    $schemaXml = Join-Path $InstallRoot "share\glib-2.0\schemas\com.gitee.gmg137.NeteaseCloudMusicGtk4.gschema.xml"
    $schemaDir = Join-Path $DistDir "share\glib-2.0\schemas"
    if (Test-Path -LiteralPath $schemaXml) {
        New-Item -ItemType Directory -Path $schemaDir -Force | Out-Null
        Copy-Item -LiteralPath $schemaXml -Destination $schemaDir -Force
        $compileSchemas = Join-Path $Prefix "bin\glib-compile-schemas.exe"
        if (-not (Test-Path -LiteralPath $compileSchemas)) {
            throw "glib-compile-schemas.exe not found: $compileSchemas"
        }
        & $compileSchemas --strict $schemaDir
        if ($LASTEXITCODE -ne 0) {
            throw "glib-compile-schemas failed with exit code $LASTEXITCODE"
        }
    }

    Write-Host "Synced exe + gresource + locale + schemas -> $DistDir"
}

$prefix = Resolve-DependencyPrefix -Explicit $DependencyPrefix
$version = Get-AppVersion
$installRoot = Join-Path $repositoryRoot "_windows\install"
$distRoot = Join-Path $repositoryRoot "_windows\dist"
$packageName = "netease-cloud-music-gtk4-$version-windows-x64"
$distDir = Join-Path $distRoot $packageName
$exePath = Join-Path $distDir "netease-cloud-music-gtk4.exe"

Write-Host "Dependency prefix: $prefix"
Write-Host "Build type: $BuildType"

Stop-AppIfRunning

& (Join-Path $PSScriptRoot "build.ps1") `
    -DependencyPrefix $prefix `
    -BuildType $BuildType
if ($LASTEXITCODE -ne 0) {
    throw "build.ps1 failed with exit code $LASTEXITCODE"
}

$needPackage = $Repackage -or -not (Test-Path -LiteralPath $exePath)
if ($needPackage) {
    Write-Host "Packaging portable tree (DLLs + plugins)..."
    & (Join-Path $PSScriptRoot "package.ps1") `
        -InstallRoot $installRoot `
        -DependencyPrefix $prefix `
        -OutputDir $distRoot `
        -Version $version
    if ($LASTEXITCODE -ne 0) {
        throw "package.ps1 failed with exit code $LASTEXITCODE"
    }
} else {
    Sync-InstallToDist -InstallRoot $installRoot -DistDir $distDir -Prefix $prefix
}

if (-not (Test-Path -LiteralPath $exePath)) {
    throw "Portable executable not found after build: $exePath"
}

if ($NoStart) {
    Write-Host "Build ready (not started): $exePath"
    Write-Output $exePath
    exit 0
}

Write-Host "Starting $exePath"
Set-Location -LiteralPath $distDir
& $exePath
exit $LASTEXITCODE
