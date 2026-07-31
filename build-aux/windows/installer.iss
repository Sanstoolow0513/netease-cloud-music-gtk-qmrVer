; Inno Setup script for the per-user Windows installer.
; All variable parts are passed by installer.ps1 via ISCC /D defines:
;   /DAppVersion /DSourceDir /DAppIco /DOutputDir /DOutputBaseFilename /DLicenseFile

#ifndef AppVersion
  #error "AppVersion is not defined. Compile through build-aux/windows/installer.ps1."
#endif
#ifndef SourceDir
  #error "SourceDir is not defined. Compile through build-aux/windows/installer.ps1."
#endif
#ifndef AppIco
  #error "AppIco is not defined. Compile through build-aux/windows/installer.ps1."
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "netease-cloud-music-gtk4-setup"
#endif
#ifndef LicenseFile
  #define LicenseFile "..\..\COPYING"
#endif

#define AppName "NetEase Cloud Music Gtk4"
#define AppExeName "netease-cloud-music-gtk4.exe"
#define AppPublisher "gmg137"
#define AppURL "https://github.com/gmg137/netease-cloud-music-gtk"

[Setup]
AppId={{E7A08FE2-0992-4B98-BC29-0235EFAC5494}}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
DefaultDirName={localappdata}\Programs\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}
SetupIconFile={#AppIco}
UninstallDisplayIcon={app}\app.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile={#LicenseFile}
CloseApplicationsFilter={#AppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "chinesesimplified"; MessagesFile: "lang\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#AppIco}"; DestDir: "{app}"; DestName: "app.ico"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\app.ico"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
