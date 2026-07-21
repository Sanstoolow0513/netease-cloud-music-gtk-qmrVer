function Initialize-NcmWindowsBuildEnvironment {
    param(
        [Parameter(Mandatory = $true)][string]$DependencyPrefix
    )

    $prefix = (Resolve-Path -LiteralPath $DependencyPrefix).Path
    $bin = Join-Path $prefix "bin"
    $lib = Join-Path $prefix "lib"
    $include = Join-Path $prefix "include"
    $pkgConfig = Join-Path $lib "pkgconfig"

    if (-not (Test-Path -LiteralPath $bin)) {
        throw "Dependency prefix is incomplete: $prefix"
    }

    $env:Path = "$bin;$env:Path"
    $env:PKG_CONFIG_PATH = "$pkgConfig;$(Join-Path $prefix 'share\pkgconfig')"
    $env:LIB = "$lib;$env:LIB"
    $env:INCLUDE = @(
        $include
        (Join-Path $include "cairo")
        (Join-Path $include "glib-2.0")
        (Join-Path $lib "glib-2.0\include")
        $env:INCLUDE
    ) -join ";"

    $env:GETTEXT_DIR = $prefix
    $env:GETTEXT_DYNAMIC = "1"
    $env:OPENSSL_DIR = $prefix
    $env:OPENSSL_LIB_DIR = $lib
    $env:OPENSSL_INCLUDE_DIR = $include
}

function Initialize-Vs2022Environment {
    if (Get-Command cl.exe -ErrorAction SilentlyContinue) {
        return
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path -LiteralPath $vswhere)) {
        throw "vswhere.exe was not found. Install Visual Studio 2022 Build Tools."
    }

    $installationPath = & $vswhere `
        -latest `
        -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
    if (-not $installationPath) {
        throw "Visual Studio 2022 C++ build tools were not found."
    }

    $devCommand = Join-Path $installationPath "Common7\Tools\VsDevCmd.bat"
    $environment = & cmd.exe /s /c "`"$devCommand`" -no_logo -arch=x64 -host_arch=x64 && set"
    foreach ($line in $environment) {
        $separator = $line.IndexOf("=")
        if ($separator -le 0) {
            continue
        }
        $name = $line.Substring(0, $separator)
        $value = $line.Substring($separator + 1)
        Set-Item -Path "Env:$name" -Value $value
    }
}
