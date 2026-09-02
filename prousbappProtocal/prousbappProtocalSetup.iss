#define MyAppName "RFID Card Encoder"
#define MyAppVersion "1.0"
#define MyAppPublisher "Your Company"
#define MyAppExeName "prousb-rfid-encoder.exe"

[Setup]
AppId={{C18E9B1A-5D42-4B9A-B5E3-725346A2C0D1}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
; Ensures the app runs correctly on both 32-bit and 64-bit Windows
ArchitecturesAllowed=x86 x64
OutputDir=installer_output
OutputBaseFilename=RFID_Encoder_Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
; This shows the icon during the installation process and on the installer .exe
SetupIconFile=prousbappProtocal\dist\prousbv10-encoder.ico

[Tasks]
; This section is REQUIRED for the "desktopicon" task to work
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Pointing to your distribution folder
Source: "prousbappProtocal\dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\proRFL.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\AESDIT.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\d12.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\d12c.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\d12c.lib"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\MFC42D.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\MSVCRTD.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\vcredist_x86.exe"; DestDir: "{tmp}"; Flags: ignoreversion
Source: "prousbappProtocal\dist\config\auth"; DestDir: "{app}\config"; Flags: ignoreversion
Source: "prousbappProtocal\dist\prousbv10-encoder.ico"; DestDir: "{app}"
; Keep this here so the icon is available in the app folder
Source: "prousbappProtocal\dist\prousbv10-encoder.ico"; DestDir: "{app}"; Flags: ignoreversion

[Run]
Filename: "{tmp}\vcredist_x86.exe"; Parameters: "/quiet /norestart"; StatusMsg: "Installing VC++ Redistributable..."

[Icons]
; Shortcut in the Start Menu group
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\prousbv10-encoder.ico"

; Shortcut on the Desktop (if the user checks the box)
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\prousbv10-encoder.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent