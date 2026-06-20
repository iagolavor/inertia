@echo off
REM Double-click to run prebuilt Inertia (no Node/Rust required).
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-desktop.ps1"
pause
