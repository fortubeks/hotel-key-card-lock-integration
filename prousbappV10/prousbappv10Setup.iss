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

[Tasks]
; This section is REQUIRED for the "desktopicon" task to work
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Pointing to your distribution folder
Source: "prousbappV10\dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\proRFL.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\AESDIT.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\d12.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\d12c.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\d12c.lib"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\MFC42D.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\MSVCRTD.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "prousbappV10\dist\vcredist_x86.exe"; DestDir: "{tmp}"; Flags: ignoreversion
Source: "prousbappV10\dist\config\auth"; DestDir: "{app}\config"; Flags: ignoreversion
Source: "prousbappV10\dist\prousbv10-encoder.ico"; DestDir: "{app}"

[Run]
Filename: "{tmp}\vcredist_x86.exe"; Parameters: "/quiet /norestart"; StatusMsg: "Installing VC++ Redistributable..."

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
; Fixed line: Using {commondesktop} is often more reliable than {autodesktop}
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent