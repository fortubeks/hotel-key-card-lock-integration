#define MyAppName "Venus ProUSB 9.27A Encoder"
#define MyAppVersion "1.2.1"
#define MyAppPublisher "Venus Hotel Software"
#define MyAppExeName "prousb-rfid-encoder.exe"

[Setup]
AppId={{427D5EC6-5CB4-49FF-A671-6BF4183B64CA}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\Venus Hotel Software\ProUSB 9.27A Encoder
DefaultGroupName={#MyAppName}
ArchitecturesAllowed=x86compatible x64compatible
OutputDir=installer_output
OutputBaseFilename=ProUSB_V9.27A_Encoder_Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
SetupIconFile=dist\prousbv10-encoder.ico

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\proRFL.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\Mwic_32.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\d12.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\d12c.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\MFC42D.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\MSVCRTD.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\config\auth"; DestDir: "{app}\config"; Flags: ignoreversion
Source: "dist\prousbv10-encoder.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\vcredist_x86.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\prousbv10-encoder.ico"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\prousbv10-encoder.ico"; Tasks: desktopicon

[Run]
Filename: "{tmp}\vcredist_x86.exe"; Parameters: "/quiet /norestart"; StatusMsg: "Installing Microsoft Visual C++ Redistributable..."; Flags: waituntilterminated
Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
