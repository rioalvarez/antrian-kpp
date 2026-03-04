@echo off
title Build Antrian Display (Tauri)
echo ========================================
echo    Building Antrian Display (Tauri)
echo ========================================
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

echo Rust version:
rustc --version
echo.

:: Check if node_modules exists
if not exist "node_modules" (
    echo Menginstall dependencies...
    call npm install
    echo.
)

:: Delete old package-lock if CLI version changed
if exist "package-lock.json" (
    echo Memperbarui Tauri CLI...
    call npm install
    echo.
)

echo Building release...
echo Ini bisa memakan waktu beberapa menit pada build pertama.
echo.
npx tauri build

echo.
echo ========================================
echo Build complete! Check:
echo   src-tauri\target\release\antrian-display.exe
echo   src-tauri\target\release\bundle\nsis\
echo ========================================
echo.
pause
