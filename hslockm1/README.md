# HS Lock SDK 2022 M1 standalone bridge

This 32-bit bridge uses the vendor's standalone M1 SDK and listens on
`127.0.0.1:9001`. It uses sector 1, matching all supplied vendor demos.

The hotel's lock number must contain six digits: two for building, two for
floor, and two for room. Only one Venus encoder bridge may run at a time.

Build with the 32-bit .NET Framework compiler:

`C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe /target:exe /platform:x86 /optimize+ /reference:System.Web.Extensions.dll /out:dist\hslock-m1-encoder.exe src\Program.cs`
