@echo off
setlocal EnableExtensions

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo [voxveil] Visual Studio Installer was not found: "%VSWHERE%"
  exit /b 1
)

set "VS_INSTALL="
for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products Microsoft.VisualStudio.Product.BuildTools -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VS_INSTALL=%%I"

if not defined VS_INSTALL (
  for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VS_INSTALL=%%I"
)

if not defined VS_INSTALL (
  echo [voxveil] No Visual Studio installation with MSVC x64/x86 tools was found.
  exit /b 1
)

set "VSDEVCMD=%VS_INSTALL%\Common7\Tools\VsDevCmd.bat"
if not exist "%VSDEVCMD%" (
  echo [voxveil] VsDevCmd.bat was not found: "%VSDEVCMD%"
  exit /b 1
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64
if errorlevel 1 exit /b %ERRORLEVEL%

where link >nul 2>&1
if errorlevel 1 (
  echo [voxveil] MSVC linker was not added to PATH by VsDevCmd.bat.
  exit /b 1
)

where cl >nul 2>&1
if errorlevel 1 (
  echo [voxveil] MSVC compiler was not added to PATH by VsDevCmd.bat.
  exit /b 1
)

echo [voxveil] MSVC environment: %VCToolsInstallDir%
call npm run dev
exit /b %ERRORLEVEL%
