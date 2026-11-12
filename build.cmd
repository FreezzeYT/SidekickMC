@echo off
setlocal
pushd "%~dp0" || exit /b 1
"%SystemRoot%\Microsoft.NET\Framework\v4.0.30319\csc.exe" /nologo /target:winexe /optimize+ /out:SidekickMC.exe /win32icon:SidekickMC.ico /reference:System.Management.dll SidekickMC.cs SettingsWindow.cs
set "result=%errorlevel%"
popd
exit /b %result%
