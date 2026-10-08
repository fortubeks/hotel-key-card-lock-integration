[Version]
Class=IEXPRESS
SEDVersion=3

[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=0
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=%InstallPrompt%
DisplayLicense=%DisplayLicense%
FinishMessage=%FinishMessage%
TargetName=installer_output\HS_Lock_SDK_2022_M1_Encoder_Setup.exe
FriendlyName=%FriendlyName%
AppLaunched=install.cmd
PostInstallCmd=<None>
AdminQuietInstCmd=install.cmd
UserQuietInstCmd=install.cmd
SourceFiles=SourceFiles

[Strings]
InstallPrompt=
DisplayLicense=
FinishMessage=HS Lock M1 encoder installed successfully.
FriendlyName=Venus HS Lock SDK 2022 M1 Encoder
FILE0=hslock-m1-encoder.exe
FILE1=auth
FILE2=libDriverWrapper_M1.dll
FILE3=libDriver_Common-0.dll
FILE4=libDriver_M1-0.dll
FILE5=libgcc_s_sjlj-1.dll
FILE6=libprotobuf-7.dll
FILE7=libstdc++-6.dll
FILE8=nspr4.dll
FILE9=install.cmd

[SourceFiles]
SourceFiles0=dist\

[SourceFiles0]
%FILE0%=
%FILE1%=
%FILE2%=
%FILE3%=
%FILE4%=
%FILE5%=
%FILE6%=
%FILE7%=
%FILE8%=
%FILE9%=
