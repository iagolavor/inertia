@echo off
REM Double-click to start API + web after setup.
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-windows.ps1"
