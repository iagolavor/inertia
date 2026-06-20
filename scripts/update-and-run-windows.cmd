@echo off
REM Update, rebuild, then start Inertia (release API + static web).
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0update-windows.ps1" -Start %*
if errorlevel 1 (
  echo.
  echo Update failed. See docs\WINDOWS-SETUP.md
  pause
  exit /b 1
)
