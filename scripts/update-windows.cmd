@echo off
REM Double-click to download latest release, rebuild, and optionally restart.
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0update-windows.ps1" %*
if errorlevel 1 (
  echo.
  echo Update failed. See docs\WINDOWS-SETUP.md
  pause
  exit /b 1
)
echo.
pause
