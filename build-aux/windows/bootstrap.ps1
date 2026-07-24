param(
    [string]$BuildRoot,
    [string]$GvsbuildVersion = "2026.4.1"
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$legacyBuildRoot = Join-Path $repositoryRoot "_windows\gvsbuild"

if (-not $BuildRoot) {
    # Keep the MSVC object tree near the drive root. Desktop/repo-relative paths
    # make webrtc/abseil exceed the classic Windows MAX_PATH limit (C1083).
    $BuildRoot = "C:\ncm-gtk"
}

if ($BuildRoot -eq $legacyBuildRoot) {
    Write-Warning "Building under the repository path can hit Windows MAX_PATH failures. Prefer C:\ncm-gtk."
}

if (
    ($BuildRoot -ne $legacyBuildRoot) -and
    (Test-Path -LiteralPath $legacyBuildRoot) -and
    -not (Test-Path -LiteralPath $BuildRoot)
) {
    Write-Host "Migrating existing gvsbuild tree from $legacyBuildRoot to $BuildRoot"
    New-Item -ItemType Directory -Path (Split-Path -Parent $BuildRoot) -Force | Out-Null
    Move-Item -LiteralPath $legacyBuildRoot -Destination $BuildRoot
}

# Meson/Ninja embed absolute tool paths. Keep a junction at the legacy location
# so previously configured projects keep working after the short-path move.
if (
    ($BuildRoot -ne $legacyBuildRoot) -and
    (Test-Path -LiteralPath $BuildRoot) -and
    -not (Test-Path -LiteralPath $legacyBuildRoot)
) {
    Write-Host "Creating compatibility junction $legacyBuildRoot -> $BuildRoot"
    New-Item -ItemType Directory -Path (Split-Path -Parent $legacyBuildRoot) -Force | Out-Null
    cmd.exe /c "mklink /J `"$legacyBuildRoot`" `"$BuildRoot`""
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to create compatibility junction for $legacyBuildRoot"
    }
}

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path -LiteralPath $cargoBin) {
    $env:Path = "$cargoBin;$env:Path"
}

if (-not (Get-Command uvx -ErrorAction SilentlyContinue)) {
    throw "uvx was not found. Install uv before bootstrapping Windows dependencies."
}
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "rustup was not found."
}

# icu (pulled in by libpsl -> libsoup3) builds its test data via `py -3`, which
# only exists with the python.org launcher. On uv-managed Python setups there is
# no `py`, so register a shim that forwards to a uv interpreter.
if (-not (Get-Command py -ErrorAction SilentlyContinue)) {
    $uvPython = (& uv python find 3 2>$null | Select-Object -First 1)
    if ($uvPython -and (Test-Path -LiteralPath $uvPython)) {
        $pyShimDir = Join-Path $BuildRoot "tools\py-shim"
        New-Item -ItemType Directory -Path $pyShimDir -Force | Out-Null
        $pyShim = Join-Path $pyShimDir "py.cmd"
        @"
@echo off
setlocal EnableDelayedExpansion
set "ARGS="
:pick
if "%~1"=="" goto run
if /I not "%~1"=="-3" set ARGS=!ARGS! "%~1"
shift
goto pick
:run
"$uvPython" !ARGS!
"@ | Set-Content -LiteralPath $pyShim -Encoding ASCII
        $env:Path = "$pyShimDir;$env:Path"
        Write-Host "Registered py shim at $pyShim -> $uvPython"
    } else {
        Write-Warning "No py launcher or uv-managed Python 3 found; the icu build may fail."
    }
}

rustup target add x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    throw "Unable to install the Rust MSVC target."
}

# gvsbuild keeps a private rustup under tools/cargo. A stale default of
# rustc 1.94.1 cannot install current cargo-c (needed by librsvg).
$gvsbuildCargoHome = Join-Path $BuildRoot "tools\cargo"
New-Item -ItemType Directory -Path $gvsbuildCargoHome -Force | Out-Null
$gvsbuildRustupSettings = Join-Path $gvsbuildCargoHome "settings.toml"
@"
version = "12"
default_toolchain = "stable-x86_64-pc-windows-msvc"
profile = "default"

[overrides]
"@ | Set-Content -LiteralPath $gvsbuildRustupSettings -Encoding ascii
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-msvc"

# OpenSSL's bundled ActivePerl build needs the VC++ 2013 runtime (MSVCR120.dll).
if (-not (Test-Path -LiteralPath "$env:SystemRoot\System32\msvcr120.dll")) {
    throw "MSVCR120.dll is missing. Install Microsoft Visual C++ 2013 Redistributable (x64) before bootstrap."
}

$sourceDir = Join-Path $BuildRoot "src"
New-Item -ItemType Directory -Path $sourceDir -Force | Out-Null

# Prefetch flaky upstream archives so gvsbuild does not keep empty/partial files.
$prefetch = @(
    @{
        Name = "cairo-1.18.4.tar.xz"
        Url = "https://cairographics.org/releases/cairo-1.18.4.tar.xz"
        Sha256 = "445ed8208a6e4823de1226a74ca319d3600e83f6369f99b14265006599c32ccb"
    }
    @{
        Name = "gstreamer-1.28.1.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/gstreamer/gstreamer-1.28.1.tar.xz"
        Sha256 = "b65e2ffa35bdbf8798cb75c23ffc3d05e484e48346ff7546844ba85217664504"
    }
    @{
        Name = "orc-0.4.42.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/orc/orc-0.4.42.tar.xz"
        Sha256 = "7ec912ab59af3cc97874c456a56a8ae1eec520c385ec447e8a102b2bd122c90c"
    }
    @{
        Name = "gst-plugins-base-1.28.1.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/gst-plugins-base/gst-plugins-base-1.28.1.tar.xz"
        Sha256 = "1446a4c2a92ff5d78d88e85a599f0038441d53333236f0c72d72f21a9c132497"
    }
    @{
        Name = "gst-plugins-good-1.28.1.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/gst-plugins-good/gst-plugins-good-1.28.1.tar.xz"
        Sha256 = "738e26aee41b7a62050e40b81adc017a110a7f32d1ec49fa6a0300846c44368d"
    }
    @{
        Name = "gst-plugins-bad-1.28.1.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/gst-plugins-bad/gst-plugins-bad-1.28.1.tar.xz"
        Sha256 = "56c1593787f8b5550893d59e4ff29e6bcccf34973316fa55e34ce493e04313a2"
    }
    @{
        Name = "gst-plugins-ugly-1.28.1.tar.xz"
        Url = "https://gstreamer.freedesktop.org/src/gst-plugins-ugly/gst-plugins-ugly-1.28.1.tar.xz"
        Sha256 = "4082f3cb063fccc3ffc04e5ab0854bafde82d1b373eb3c9eaa28115dd3f95a78"
    }
)

function Ensure-PrefetchedArchive {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Item
    )

    $path = Join-Path $sourceDir $Item.Name
    if (Test-Path -LiteralPath $path) {
        $existing = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLower()
        if ($existing -eq $Item.Sha256.ToLower() -and (Get-Item -LiteralPath $path).Length -gt 0) {
            Write-Host "Using cached $($Item.Name)"
            return
        }
        Write-Host "Removing invalid cache for $($Item.Name)"
        Remove-Item -LiteralPath $path -Force
    }

    Write-Host "Downloading $($Item.Name)"
    curl.exe -L --fail --retry 5 --retry-all-errors --retry-delay 2 -o $path $Item.Url
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to download $($Item.Url)"
    }

    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLower()
    if ($hash -ne $Item.Sha256.ToLower()) {
        throw "Hash mismatch for $($Item.Name): calculated $hash, expected $($Item.Sha256)"
    }
}

foreach ($item in $prefetch) {
    Ensure-PrefetchedArchive -Item $item
}

# Keep the GStreamer set focused on playback. gstreamer-all also pulls tooling
# packages that are unnecessary for the portable MVP and enlarge CI time.
$projects = @(
    "gtk4"
    "libadwaita"
    "adwaita-icon-theme"
    "openssl"
    "gstreamer"
    "gst-plugins-base"
    # glib-networking + libsoup3 must precede gst-plugins-good: its soup plugin
    # (souphttpsrc) is an auto feature and is silently skipped without libsoup3.
    "glib-networking"
    "libsoup3"
    "gst-plugins-good"
    "gst-plugins-bad"
    "gst-plugins-ugly"
    # gst-libav (ffmpeg) provides mp3/flac/aac decoders; without it only the
    # Media Foundation mp3 decoder is available and flac/hires cannot play.
    "gst-libav"
)

# Skip webrtc-audio-processing: only used by webrtcdsp, and its abseil tree is a
# frequent Windows path-length failure point. Clean any incomplete leftover first.
$webrtcBuildDir = Join-Path $BuildRoot "build\x64\release\webrtc-audio-processing"
if (Test-Path -LiteralPath $webrtcBuildDir) {
    Write-Host "Cleaning incomplete webrtc-audio-processing build at $webrtcBuildDir"
    Remove-Item -LiteralPath $webrtcBuildDir -Recurse -Force
}

# --fast-build skips projects that already completed. gst-plugins-good was first
# built without libsoup3, so force a rebuild until gstsoup.dll shows up; without
# souphttpsrc the player cannot open any http(s) stream.
$soupPlugin = Join-Path $BuildRoot "gtk\x64\release\lib\gstreamer-1.0\gstsoup.dll"
$goodBuildDir = Join-Path $BuildRoot "build\x64\release\gst-plugins-good"
if ((Test-Path -LiteralPath $goodBuildDir) -and -not (Test-Path -LiteralPath $soupPlugin)) {
    Write-Host "Rebuilding gst-plugins-good so the soup plugin picks up libsoup3..."
    Remove-Item -LiteralPath $goodBuildDir -Recurse -Force
}

# GitHub Windows runners ship both Git Bash and MSYS2; Git's usr\bin often wins
# on PATH. libvpx's configure then probes /tmp with Git's cat/mv against a
# different tmp root and fails before producing vpxmd.lib (wingtk/gvsbuild#1723).
# Prefer system MSYS2 and pass --use-env so gvsbuild keeps that PATH.
$msysUsrBin = "C:\msys64\usr\bin"
if (Test-Path -LiteralPath $msysUsrBin) {
    $env:Path = "$msysUsrBin;$env:Path"
    Write-Host "Preferring MSYS2 tools at $msysUsrBin for gvsbuild"
} else {
    Write-Warning "C:\msys64\usr\bin not found; libvpx may fail if Git Bash tools shadow MSYS2."
}

# gvsbuild's stock ffmpeg build.sh cannot produce the decoders we need and its
# configure probe breaks on non-English VS installs (localized cl banner ->
# "Unknown C compiler" -> MSVC ignores the resulting `-o` flag). Swap in the
# repo-maintained ffmpeg-build.sh before building. uvx runs gvsbuild straight
# from the uv cache, so overwrite the cached patch file.
& uvx --from "gvsbuild==$GvsbuildVersion" gvsbuild --help | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Unable to prime the gvsbuild $GvsbuildVersion environment."
}
# setup-uv on GitHub Actions sets UV_CACHE_DIR (e.g. D:\a\_temp\setup-uv-cache);
# uvx then extracts there, not under %LOCALAPPDATA%\uv\cache.
$ffmpegBuildSh = $null
$uvCacheRoot = if ($env:UV_CACHE_DIR) { $env:UV_CACHE_DIR } else { Join-Path $env:LOCALAPPDATA "uv\cache" }
$uvArchiveCache = Join-Path $uvCacheRoot "archive-v0"
if (Test-Path -LiteralPath $uvArchiveCache) {
    $ffmpegBuildSh = Get-ChildItem -LiteralPath $uvArchiveCache -Directory |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "Lib\site-packages\gvsbuild-$GvsbuildVersion.dist-info") } |
        ForEach-Object { Join-Path $_.FullName "Lib\site-packages\gvsbuild\patches\ffmpeg\build\build.sh" } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
    # uv also keeps a flat wheel extract (no Lib\site-packages); prefer the
    # environment copy above, but fall back so CI/local layouts both work.
    if (-not $ffmpegBuildSh) {
        $ffmpegBuildSh = Get-ChildItem -LiteralPath $uvArchiveCache -Directory |
            Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "gvsbuild-$GvsbuildVersion.dist-info") } |
            ForEach-Object { Join-Path $_.FullName "gvsbuild\patches\ffmpeg\build\build.sh" } |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
    }
}
if (-not $ffmpegBuildSh) {
    throw "Unable to locate gvsbuild $GvsbuildVersion ffmpeg build.sh under $uvArchiveCache; cannot apply the decoder/locale fixes."
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "ffmpeg-build.sh") -Destination $ffmpegBuildSh -Force
if (-not (Select-String -LiteralPath $ffmpegBuildSh -Pattern "netease-cloud-music-gtk4" -Quiet)) {
    throw "Failed to install the customized ffmpeg build.sh at $ffmpegBuildSh"
}
Write-Host "Patched gvsbuild ffmpeg build.sh at $ffmpegBuildSh"

# The patches directory is only copied on fresh extraction; an already
# extracted ffmpeg tree keeps its old build.sh, so overwrite that copy too.
$extractedBuildSh = Join-Path $BuildRoot "build\x64\release\ffmpeg\build\build.sh"
if (Test-Path -LiteralPath $extractedBuildSh) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "ffmpeg-build.sh") -Destination $extractedBuildSh -Force
    Write-Host "Patched extracted ffmpeg build.sh at $extractedBuildSh"
}

& uvx --from "gvsbuild==$GvsbuildVersion" gvsbuild build `
    --build-dir $BuildRoot `
    --platform x64 `
    --configuration release `
    --vs-ver vs2022 `
    --fast-build `
    --use-env `
    --skip webrtc-audio-processing `
    --extra-opts "gst-plugins-bad:-Dwebrtcdsp=disabled" `
    --extra-opts "gst-plugins-good:-Dsoup=enabled" `
    @projects
if ($LASTEXITCODE -ne 0) {
    throw "gvsbuild failed with exit code $LASTEXITCODE"
}

$dependencyPrefix = Join-Path $BuildRoot "gtk\x64\release"
$gtkRuntime = Get-ChildItem `
    -LiteralPath (Join-Path $dependencyPrefix "bin") `
    -Filter "gtk-4*.dll" `
    -ErrorAction SilentlyContinue
if (-not $gtkRuntime) {
    throw "GTK runtime was not produced at $dependencyPrefix"
}

$gstRuntime = Get-ChildItem `
    -LiteralPath (Join-Path $dependencyPrefix "bin") `
    -Filter "gstreamer-1.0-0.dll" `
    -ErrorAction SilentlyContinue
if (-not $gstRuntime) {
    throw "GStreamer runtime was not produced at $dependencyPrefix"
}

# Cargo links against these .pc files; missing play usually means gst-plugins-bad
# did not finish. Fail early with a recoverable message instead of a vague meson error.
$requiredPkgConfig = @(
    "gtk4.pc"
    "libadwaita-1.pc"
    "gstreamer-1.0.pc"
    "gstreamer-base-1.0.pc"
    "gstreamer-audio-1.0.pc"
    "gstreamer-play-1.0.pc"
    "openssl.pc"
)
$pkgConfigDir = Join-Path $dependencyPrefix "lib\pkgconfig"
$missingPkgConfig = @()
foreach ($pc in $requiredPkgConfig) {
    if (-not (Test-Path -LiteralPath (Join-Path $pkgConfigDir $pc))) {
        $missingPkgConfig += $pc
    }
}
if ($missingPkgConfig.Count -gt 0) {
    throw @"
Dependency prefix is incomplete at $dependencyPrefix
Missing pkg-config files: $($missingPkgConfig -join ', ')
Re-run this script after fixing the failed gvsbuild project. Completed projects are skipped with --fast-build.
"@
}

# Playback-critical plugins: souphttpsrc fetches the stream, libav decodes
# mp3/flac/aac. Their meson features auto-disable silently when a dependency is
# missing, so verify the artifacts instead of trusting a green build.
$requiredPlugins = @("gstsoup.dll", "gstlibav.dll")
$pluginDir = Join-Path $dependencyPrefix "lib\gstreamer-1.0"
$missingPlugins = @()
foreach ($plugin in $requiredPlugins) {
    if (-not (Test-Path -LiteralPath (Join-Path $pluginDir $plugin))) {
        $missingPlugins += $plugin
    }
}
if ($missingPlugins.Count -gt 0) {
    throw @"
Dependency prefix is missing playback plugins: $($missingPlugins -join ', ')
Audio playback will fail without them. Delete the matching project directory under
$BuildRoot\build\x64\release and re-run this script.
"@
}

# Without the glib-networking OpenSSL GIO module, https streams cannot be opened.
$gioModuleDir = Join-Path $dependencyPrefix "lib\gio\modules"
$gioModule = Get-ChildItem -LiteralPath $gioModuleDir -Filter "*gioopenssl.dll" -ErrorAction SilentlyContinue
if (-not $gioModule) {
    throw @"
Dependency prefix is missing the glib-networking TLS module under $gioModuleDir
https playback will fail without it. Delete $BuildRoot\build\x64\release\glib-networking
and re-run this script.
"@
}

Write-Host "Windows dependency prefix is ready: $dependencyPrefix"
Write-Output $dependencyPrefix
