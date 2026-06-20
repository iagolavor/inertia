@echo off
REM One-shot: install Node/Rust/Git via winget (when missing), then build.
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-windows.ps1" -InstallDeps %*
if errorlevel 1 (
  echo.
  echo Setup failed. See docs\WINDOWS-SETUP.md
  pause
  exit /b 1
)
echo.
pause
