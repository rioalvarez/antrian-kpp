@echo off
title Build Antrian Tiket (Tauri)
echo ========================================
echo    Building Antrian Tiket (Tauri)
echo ========================================
echo.

where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust belum terinstall!
    echo Download di: https://rustup.rs
    echo.
    pause
    exit /b 1
)

if not exist "node_modules" (
    echo Menginstall dependencies...
    call npm install
    echo.
)

echo Building release...
npx tauri build

echo.
echo ========================================
echo Build complete! Check:
echo   src-tauri\target\release\
echo   src-tauri\target\release\bundle\nsis\
echo ========================================
echo.
pause
