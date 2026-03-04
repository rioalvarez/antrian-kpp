@echo off
title Antrian Display (Tauri Dev)
echo Menjalankan Display Antrian (Tauri Dev Mode)...
echo.

:: Check prerequisites
where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust belum terinstall!
    echo Download di: https://rustup.rs
    echo.
    pause
    exit /b 1
)

:: Check if node_modules exists
if not exist "node_modules" (
    echo Menginstall dependencies...
    call npm install
    echo.
)

echo Pastikan server antrian sudah berjalan di http://localhost:8080
echo.

npx tauri dev
