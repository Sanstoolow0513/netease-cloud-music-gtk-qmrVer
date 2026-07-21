param(
    [Parameter(Mandatory = $true)][string]$DependencyPrefix,
    [string]$BuildDir,
    [string]$InstallRoot,
    [ValidateSet("debug", "release")][string]$BuildType = "release",
    [switch]$Package
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

if (-not $BuildDir) {
    $BuildDir = Join-Path $repositoryRoot "_windows\build"
}
if (-not $InstallRoot) {
    $InstallRoot = Join-Path $repositoryRoot "_windows\install"
}

. (Join-Path $PSScriptRoot "env.ps1")
Initialize-Vs2022Environment
Initialize-NcmWindowsBuildEnvironment -DependencyPrefix $DependencyPrefix
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-msvc"
$uvToolBin = Join-Path $env:USERPROFILE ".local\bin"
if (Test-Path -LiteralPath $uvToolBin) {
    $env:Path = "$uvToolBin;$env:Path"
}
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path -LiteralPath $cargoBin) {
    $env:Path = "$cargoBin;$env:Path"
}

foreach ($command in @("meson", "ninja", "pkg-config", "cargo")) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "$command was not found in PATH."
    }
}

foreach ($dependency in @("gtk4", "libadwaita-1", "gstreamer-1.0", "gstreamer-play-1.0")) {
    & pkg-config --exists $dependency
    if ($LASTEXITCODE -ne 0) {
        throw "pkg-config could not resolve $dependency from $DependencyPrefix"
    }
}

$setupArgs = @(
    "setup"
    $BuildDir
    "--prefix=$InstallRoot"
    "--buildtype=$BuildType"
)
if (Test-Path -LiteralPath (Join-Path $BuildDir "build.ninja")) {
    $setupArgs += "--reconfigure"
}

& meson @setupArgs
if ($LASTEXITCODE -ne 0) {
    throw "Meson setup failed with exit code $LASTEXITCODE"
}

& meson compile -C $BuildDir
if ($LASTEXITCODE -ne 0) {
    throw "Meson compile failed with exit code $LASTEXITCODE"
}

& meson install -C $BuildDir
if ($LASTEXITCODE -ne 0) {
    throw "Meson install failed with exit code $LASTEXITCODE"
}

if ($Package) {
    & (Join-Path $PSScriptRoot "package.ps1") `
        -InstallRoot $InstallRoot `
        -DependencyPrefix $DependencyPrefix
}
