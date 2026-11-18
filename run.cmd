@echo off
setlocal EnableExtensions

rem Always run from the repository root, even when invoked by full path.
pushd "%~dp0" || exit /b 1

rem Locate the MinGW linker installed by winget. Its package directory name
rem includes a source suffix, so match it instead of hardcoding the path.
for /d %%D in ("%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_*") do (
    if exist "%%~D\mingw64\bin\gcc.exe" set "PATH=%%~D\mingw64\bin;%PATH%"
)

if /I "%~1"=="gui" (
    cargo +stable-x86_64-pc-windows-gnu run -p ebd-ui
) else (
    cargo +stable-x86_64-pc-windows-gnu run -p ebd-cli -- %*
)
set "STATUS=%ERRORLEVEL%"

popd
if not "%STATUS%"=="0" echo run.cmd failed with exit code %STATUS%.
exit /b %STATUS%
