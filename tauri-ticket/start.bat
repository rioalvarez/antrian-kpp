@echo off
title Antrian Tiket (Tauri)
echo Menjalankan Aplikasi Tiket Antrian (Tauri)...
echo.

if not exist "node_modules" (
    echo Menginstall dependencies...
    call npm install
    echo.
)

npx tauri dev
