@echo off
title Build Antrian KPP - Server Release
echo ================================================
echo   Build Antrian KPP - Server Release
echo ================================================
echo.

:: Destination folder
set RELEASE_DIR=release
if exist %RELEASE_DIR% rmdir /s /q %RELEASE_DIR%
mkdir %RELEASE_DIR%\server
mkdir %RELEASE_DIR%\server\data

echo.
echo Pilih opsi database:
echo [1] Tanpa database (fresh install)
echo [2] Sertakan database development
echo.
set /p BUILD_OPTION="Pilihan (1/2): "

echo.
echo ================================================
echo [1/3] Memeriksa file audio...
echo ================================================

:: Audio di web/static/audio hanya diperlukan jika mode "Audio Server" (server_audio)
:: diaktifkan di panel admin display. Mode lain (local_tts, web_speech) tidak membutuhkan
:: file ini — file audio tetap di-embed ke binary jika ada, tapi tidak wajib.
set AUDIO_OK=1
if not exist web\static\audio\nomor_antrian.mp3 set AUDIO_OK=0
if not exist web\static\audio\angka_1.mp3        set AUDIO_OK=0
if not exist web\static\audio\huruf_a.mp3         set AUDIO_OK=0

if %AUDIO_OK%==0 (
    echo [INFO] File audio tidak ditemukan di web\static\audio\
    echo.
    echo        File audio hanya diperlukan jika mode "Audio Server"
    echo        ^(server_audio^) diaktifkan di panel admin display.
    echo        Mode lain ^(local_tts / web_speech^) tidak membutuhkan file ini.
    echo.
    echo        Jika Anda berencana menggunakan mode server_audio,
    echo        pastikan semua file MP3 sudah ada sebelum build.
    echo.
    set /p AUDIO_CONFIRM="Lanjutkan build tanpa file audio? (y/n): "
    if /i not "%AUDIO_CONFIRM%"=="y" (
        echo Build dibatalkan.
        pause
        exit /b 1
    )
) else (
    for /f %%A in ('dir /b /a-d "web\static\audio\*.mp3" 2^>nul ^| find /c /v ""') do set AUDIO_COUNT=%%A
    echo [OK] %AUDIO_COUNT% file audio ditemukan di web\static\audio\
    echo      File audio di-embed ke binary ^(digunakan saat mode "Audio Server"^).
)

echo.
echo ================================================
echo [2/3] Building server...
echo ================================================

:: Cek Go terinstall
where go >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Go tidak ditemukan!
    echo Install dari https://go.dev/dl/
    pause
    exit /b 1
)

:: Build binary (go:embed menyertakan web/ + audio ke dalam exe)
echo Compiling...
go build -ldflags="-s -w" -o %RELEASE_DIR%\server\antrian-kpp.exe .
if %ERRORLEVEL% neq 0 (
    echo ERROR: Build gagal!
    pause
    exit /b 1
)
echo [OK] antrian-kpp.exe berhasil dibuat
echo      (web assets + audio sudah ter-embed di dalam binary)

:: Copy config jika ada
if exist config.yaml (
    copy /Y config.yaml %RELEASE_DIR%\server\ >nul
    echo [OK] config.yaml disertakan
) else (
    echo [INFO] config.yaml tidak ditemukan, akan dibuat otomatis saat server pertama kali dijalankan
)

:: Copy database sesuai pilihan
if "%BUILD_OPTION%"=="2" (
    if exist data\queue.db (
        copy /Y data\queue.db %RELEASE_DIR%\server\data\ >nul
        echo [OK] Database queue.db disertakan
    ) else (
        echo [WARNING] data\queue.db tidak ditemukan, database akan dibuat fresh
    )
) else (
    echo [INFO] Database akan dibuat otomatis saat server pertama kali dijalankan
)

:: Buat start-server.bat
(
echo @echo off
echo title Antrian KPP Server
echo echo ========================================
echo echo   Antrian KPP Server
echo echo ========================================
echo echo.
echo echo Starting server...
echo echo Akses dari komputer ini   : http://localhost:8080
echo echo Akses dari komputer lain  : http://[IP_ADDRESS]:8080
echo echo.
echo echo Tekan Ctrl+C untuk menghentikan server
echo echo.
echo antrian-kpp.exe
echo pause
) > %RELEASE_DIR%\server\start-server.bat
echo [OK] start-server.bat dibuat

echo.
echo ================================================
echo [3/3] Membuat dokumentasi...
echo ================================================

(
echo ================================================================================
echo                    ANTRIAN KPP - SERVER RELEASE PACKAGE
echo ================================================================================
echo.
echo STRUKTUR FOLDER:
echo.
echo   release/
echo   └── server/
echo       ├── antrian-kpp.exe     ^<-- Jalankan file ini
echo       ├── start-server.bat    ^<-- Atau klik file ini
echo       ├── config.yaml         ^<-- Konfigurasi server
echo       └── data/
echo           └── queue.db        ^<-- Database ^(dibuat otomatis^)
echo.
echo CATATAN:
echo   Binary sudah menyertakan semua web assets dan file audio.
echo   Tidak diperlukan folder tambahan selain yang ada di atas.
echo.
echo ================================================================================
echo                         PANDUAN INSTALASI SERVER
echo ================================================================================
echo.
echo 1. Copy folder 'server' ke komputer server
echo 2. Double-click 'start-server.bat' untuk menjalankan
echo 3. Buka Windows Firewall - izinkan port 8080
echo 4. Catat IP address server ^(jalankan: ipconfig^)
echo.
echo ================================================================================
echo                    PANDUAN KONFIGURASI CLIENT
echo ================================================================================
echo.
echo Setiap perangkat client ^(display, loket, mesin tiket^) perlu dikonfigurasi
echo dengan IP address server melalui aplikasi Tauri masing-masing.
echo.
echo Aplikasi client diinstall terpisah dari package ini.
echo.
echo ================================================================================
echo                           TROUBLESHOOTING
echo ================================================================================
echo.
echo Server tidak bisa diakses dari client:
echo   - Pastikan server sudah berjalan ^(cek jendela console^)
echo   - Buka Windows Firewall - izinkan antrian-kpp.exe atau port 8080
echo   - Pastikan semua perangkat dalam jaringan yang sama
echo   - Cek IP address server dengan perintah: ipconfig
echo.
echo Audio tidak berbunyi:
echo   - Buka panel admin ^(/admin^) lalu pilih mode audio di pengaturan display:
echo     * local_tts  : audio MP3 bawaan aplikasi Tauri di perangkat display
echo     * server_audio : audio MP3 yang di-embed di dalam binary server
echo     * web_speech : TTS bawaan browser ^(tidak perlu file audio^)
echo   - Cek volume di komputer display ^(bukan server^)
echo.
echo ================================================================================
) > %RELEASE_DIR%\README.txt
echo [OK] README.txt dibuat

echo.
echo ================================================
echo           BUILD SELESAI!
echo ================================================
echo.
echo Package tersimpan di: %RELEASE_DIR%\server\
echo.
echo   antrian-kpp.exe     ^(binary server, sudah termasuk semua assets^)
echo   start-server.bat
echo   config.yaml
if "%BUILD_OPTION%"=="2" (
echo   data\queue.db       ^(DATABASE DISERTAKAN^)
) else (
echo   data\               ^(kosong, database dibuat saat server jalan^)
)
echo.
pause
