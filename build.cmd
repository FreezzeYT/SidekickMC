@echo off
setlocal
pushd "%~dp0" || exit /b 1
"%SystemRoot%\Microsoft.NET\Framework\v4.0.30319\csc.exe" /nologo /target:winexe /optimize+ /out:MouseControls.exe /win32icon:MouseControls.ico /reference:System.Management.dll MouseControls.cs SettingsWindow.cs
set "result=%errorlevel%"
popd
exit /b %result%
