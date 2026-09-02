[Setup]
SetupIconFile=rfid-encoder.ico
AppName=RFID Encoder
AppVersion=1.0
DefaultDirName={pf}\ RFID Encoder
DefaultGroupName= RFID Encoder
UninstallDisplayIcon={app}\rfid-encoder.exe
OutputDir=.
OutputBaseFilename=RFID EncoderInstaller
Compression=lzma
SolidCompression=yes

[Files]
Source: "rfidEncoder\dist\rfidEncoder.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\DataReader.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\Defines.h"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\des.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\EasyD12_500.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\EasyZUSBMulti.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\HOOKS_M1.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\HSDApp.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockCard.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockInfo.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockReg.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockSDK.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockSDK.h"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\LockSDK.lib"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\MF0SIM.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\MFC42D.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\MSVCRTD.DLL"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\PubFuns.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\RC500USB.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\Rf_Rw.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\RF50S.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\RF57S.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\run_server.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "rfidEncoder\dist\config\auth"; DestDir: "{app}\config"; Flags: ignoreversion
Source: "rfidEncoder\dist\rfid-encoder.ico"; DestDir: "{app}"


[Run]
;Filename: "{tmp}\vcredist_x86.exe"; Parameters: "/quiet /norestart"; StatusMsg: "Installing VC++ Redistributable..."

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"


[Icons]
Name: "{group}\RFID Encoder"; Filename: "{app}\run_server.bat"; IconFilename: "{app}\rfid-encoder.ico"
Name: "{commondesktop}\RFID Encoder"; Filename: "{app}\run_server.bat"; Tasks: desktopicon; IconFilename: "{app}\rfid-encoder.ico"
Name: "{group}\Uninstaller"; Filename: "{uninstallexe}"
