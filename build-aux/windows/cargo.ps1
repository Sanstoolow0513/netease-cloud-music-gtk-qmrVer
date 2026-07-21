param(
    [Parameter(Mandatory = $true)][string]$MesonBuildRoot,
    [Parameter(Mandatory = $true)][string]$MesonSourceRoot,
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][string]$BuildType,
    [Parameter(Mandatory = $true)][string]$AppBin
)

$ErrorActionPreference = "Stop"
$env:MESON_BUILD_ROOT = $MesonBuildRoot
$env:MESON_SOURCE_ROOT = $MesonSourceRoot
$env:CARGO_TARGET_DIR = Join-Path $MesonBuildRoot "target"

$cargoArgs = @(
    "build",
    "--manifest-path", (Join-Path $MesonSourceRoot "Cargo.toml")
)
$profile = "debug"
if ($BuildType -eq "release") {
    $cargoArgs += "--release"
    $profile = "release"
}

& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    throw "Cargo build failed with exit code $LASTEXITCODE"
}

$executable = Join-Path $env:CARGO_TARGET_DIR "$profile\$AppBin.exe"
Copy-Item -LiteralPath $executable -Destination $Output -Force
