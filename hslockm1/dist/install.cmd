@echo off
setlocal
set "INSTALL_DIR=%LOCALAPPDATA%\Venus Hotel Software\HS Lock M1 Encoder"
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
for %%F in (hslock-m1-encoder.exe auth libDriverWrapper_M1.dll libDriver_Common-0.dll libDriver_M1-0.dll libgcc_s_sjlj-1.dll libprotobuf-7.dll libstdc++-6.dll nspr4.dll) do copy /Y "%~dp0%%F" "%INSTALL_DIR%\%%F" >nul
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$s=(New-Object -ComObject WScript.Shell).CreateShortcut([Environment]::GetFolderPath('Desktop')+'\Venus HS Lock M1 Encoder.lnk');$s.TargetPath=$env:LOCALAPPDATA+'\Venus Hotel Software\HS Lock M1 Encoder\hslock-m1-encoder.exe';$s.WorkingDirectory=$env:LOCALAPPDATA+'\Venus Hotel Software\HS Lock M1 Encoder';$s.Save()"
start "" "%INSTALL_DIR%\hslock-m1-encoder.exe"
endlocal
