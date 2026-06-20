@echo off
REM Double-click or run from cmd — no PowerShell execution-policy change needed.
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-windows.ps1" %*
if errorlevel 1 (
  echo.
  echo Setup failed. See docs\WINDOWS-SETUP.md
  pause
  exit /b 1
)
echo.
pause
