@echo off
title Antrian Loket (Tauri)
echo Menjalankan Aplikasi Loket (Tauri)...
echo.

:: Check if node_modules exists
if not exist "node_modules" (
    echo Menginstall dependencies...
    call npm install
    echo.
)

npx tauri dev
