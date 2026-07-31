# Build the Inno Setup installer from an already-packaged portable staging tree.
# Run package.ps1 (or build.ps1 -Package / -Installer) first to produce the staging dir.
# Usage:
#   .\build-aux\windows\installer.ps1
#   .\build-aux\windows\installer.ps1 -StagingDir _windows\dist\netease-cloud-music-gtk4-2.5.3-windows-x64
# The last line of output is the produced setup exe path (same convention as package.ps1).
param(
    [string]$StagingDir,
    [string]$OutputDir,
    [string]$Version,
    [string]$IsccPath
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

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
if (-not $StagingDir) {
    $StagingDir = Join-Path $OutputDir "netease-cloud-music-gtk4-$Version-windows-x64"
}
$staging = (Resolve-Path -LiteralPath $StagingDir).Path
if (-not (Test-Path -LiteralPath (Join-Path $staging "netease-cloud-music-gtk4.exe"))) {
    throw "Staging dir does not contain netease-cloud-music-gtk4.exe: $staging`nRun package.ps1 first (or build.ps1 -Package / -Installer)."
}
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

function Resolve-Iscc {
    param([string]$Explicit)
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) {
            throw "ISCC.exe not found: $Explicit"
        }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    $fromPath = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }
    foreach ($candidate in @(
            (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe")
            "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
            "C:\Program Files\Inno Setup 6\ISCC.exe"
        )) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }
    throw @"
Inno Setup 6 (ISCC.exe) was not found.
Install it with one of:
  winget install JRSoftware.InnoSetup
  choco install innosetup
or download from https://jrsoftware.org/isdl.php
"@
}

# Render data/icons/hicolor/512x512@2x.png into a multi-size BMP-frame .ico
# (BMP frames instead of PNG-compressed frames for maximum ISCC/Windows compat).
function New-AppIcon {
    param([string]$SourcePng, [string]$DestIco)

    Add-Type -AssemblyName System.Drawing
    $src = [System.Drawing.Image]::FromFile($SourcePng)
    $sizes = @(16, 24, 32, 48, 64, 128, 256)
    $frames = @()
    foreach ($size in $sizes) {
        $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.Clear([System.Drawing.Color]::Transparent)
        $g.DrawImage($src, 0, 0, $size, $size)
        $g.Dispose()

        $bmpStream = New-Object System.IO.MemoryStream
        $bmp.Save($bmpStream, [System.Drawing.Imaging.ImageFormat]::Bmp)
        $bmp.Dispose()
        $bmpBytes = $bmpStream.ToArray()
        $bmpStream.Dispose()

        # ICO BMP frame = BMP without its 14-byte BITMAPFILEHEADER, biHeight doubled,
        # followed by a zero AND mask (1bpp, 4-byte-aligned stride; alpha comes from XOR data).
        $maskStride = [int][math]::Floor(($size + 31) / 32) * 4
        $frameStream = New-Object System.IO.MemoryStream
        $frameStream.Write($bmpBytes, 14, $bmpBytes.Length - 14)
        $frameStream.Write((New-Object byte[] ($maskStride * $size)), 0, $maskStride * $size)
        $frame = $frameStream.ToArray()
        $frameStream.Dispose()
        $heightBytes = [BitConverter]::GetBytes([int]($size * 2))
        for ($i = 0; $i -lt 4; $i++) {
            $frame[8 + $i] = $heightBytes[$i]
        }
        $frames += , $frame
    }
    $src.Dispose()

    $destDir = Split-Path -Parent $DestIco
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    $fs = New-Object System.IO.FileStream($DestIco, [System.IO.FileMode]::Create)
    $bw = New-Object System.IO.BinaryWriter($fs)
    $bw.Write([uint16]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]$frames.Count)
    $offset = 6 + 16 * $frames.Count
    for ($i = 0; $i -lt $frames.Count; $i++) {
        $bw.Write([byte]($sizes[$i] % 256))
        $bw.Write([byte]($sizes[$i] % 256))
        $bw.Write([byte]0)
        $bw.Write([byte]0)
        $bw.Write([uint16]1)
        $bw.Write([uint16]32)
        $bw.Write([uint32]$frames[$i].Length)
        $bw.Write([uint32]$offset)
        $offset += $frames[$i].Length
    }
    foreach ($frame in $frames) {
        $bw.Write($frame)
    }
    $bw.Dispose()
    $fs.Dispose()
}

$iscc = Resolve-Iscc -Explicit $IsccPath

$appIco = Join-Path $repositoryRoot "_windows\build\installer-assets\app.ico"
New-AppIcon `
    -SourcePng (Join-Path $repositoryRoot "data\icons\hicolor\512x512@2x.png") `
    -DestIco $appIco

$issFile = Join-Path $PSScriptRoot "installer.iss"
$outputBaseName = "netease-cloud-music-gtk4-$Version-windows-x64-setup"
$setupExe = Join-Path $OutputDir "$outputBaseName.exe"
if (Test-Path -LiteralPath $setupExe) {
    Remove-Item -LiteralPath $setupExe -Force
}

& $iscc `
    "/DAppVersion=$Version" `
    "/DSourceDir=$staging" `
    "/DAppIco=$appIco" `
    "/DOutputDir=$OutputDir" `
    "/DOutputBaseFilename=$outputBaseName" `
    "/DLicenseFile=$repositoryRoot\COPYING" `
    $issFile
if ($LASTEXITCODE -ne 0) {
    throw "ISCC.exe failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path -LiteralPath $setupExe)) {
    throw "The installer was not produced: $setupExe"
}

Write-Output $setupExe
