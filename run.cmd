@echo off
setlocal EnableExtensions

rem Use the MinGW linker installed alongside the GNU Rust toolchain.
rem The winget package directory name includes a source suffix, so match it.
for /d %%D in ("%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_*") do (
    if exist "%%~D\mingw64\bin\gcc.exe" set "PATH=%%~D\mingw64\bin;%PATH%"
)

cargo +stable-x86_64-pc-windows-gnu run -p ebd-cli -- %*
exit /b %ERRORLEVEL%
