# HS Lock SDK 2022 M1 standalone bridge

This 32-bit bridge uses the vendor's standalone M1 SDK and listens on
`127.0.0.1:9001`. Reading an existing encoded card scans sectors 1 through 15,
saves the detected sector in `sector.cfg`, and reuses it for writes and clears.

The packaged M1 native runtime is taken from the working HS Lock 11.0.1
installation supplied for this integration. Keep the complete matching runtime
set together; its `libDriver_M1-0.dll` is not byte-compatible with the generic
SDK 2022 demonstration runtime. The Java application, configuration, and hotel
database are intentionally excluded.

At startup the bridge calls the runtime's required `BuildDriver()` initializer,
matching the sequence used by `CardDriverForM1` in the working Java application.
It calls `DestroyDriver()` when the process exits normally.

The hotel's lock number must contain six digits: two for building, two for
floor, and two for room. Only one Venus encoder bridge may run at a time.

Build with the 32-bit .NET Framework compiler:

`C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe /target:exe /platform:x86 /optimize+ /reference:System.Web.Extensions.dll /out:dist\hslock-m1-encoder.exe src\Program.cs`
