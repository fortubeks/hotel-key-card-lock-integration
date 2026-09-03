# ProUSB 9.27A encoder bridge

This package uses the 32-bit `proRFL.dll` runtime collected from a computer where
ProUSB 9.27A successfully writes guest cards. The release executable is compiled
from `src/Program.cs` as a 32-bit .NET Framework application. The original Rust
prototype remains in `src/main.rs` for reference.

Do not add `CardLock.mdb`, `System.ini`, or the original ProUSB application to
the Venus installer. Those files contain hotel-specific application data and
are not required by the bridge.

Build `prousbappv927aSetup.iss` with Inno Setup 6. The resulting installer is:

`installer_output/ProUSB_V9.27A_Encoder_Setup.exe`

Only one Venus encoder bridge should be running because all bridge variants
listen on `127.0.0.1:9001`.

Version 1.2.1 correctly derives `dlsCoID` from characters 9-14 of an existing
encoded card. It also accepts the hotel's configured numeric `dlsCoID`. Existing guest
cards are validated against that ID before they can be changed, blank cards use
the configured ID, and every successful write is read back and decoded before
the API reports success.

The pre-write restriction that allowed only SDK card types `6` and `F` is
temporarily disabled for 9.27A compatibility. The post-write verification is
still required before the API reports success.

Compile the bridge before building the installer:

`C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe /target:exe /platform:x86 /optimize+ /reference:System.Web.Extensions.dll /out:dist\prousb-rfid-encoder.exe src\Program.cs`
