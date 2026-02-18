# Sistem Antrian KPP Pratama

Aplikasi manajemen antrian berbasis web untuk lingkungan kantor pelayanan publik. Dibangun dengan Go dan berjalan sepenuhnya secara lokal — tidak memerlukan koneksi internet maupun server eksternal.

Sistem ini dirancang untuk dioperasikan dalam jaringan **intranet/LAN/WiFi kantor**, sehingga seluruh perangkat (monitor display, komputer loket, mesin cetak tiket) dapat terhubung dan tersinkronisasi secara real-time hanya dengan browser.

---

## Fitur Utama

- **Antrian real-time** — update otomatis ke semua perangkat menggunakan Server-Sent Events (SSE), tanpa perlu refresh halaman
- **Multi-jenis antrian** — mendukung beberapa jenis layanan dengan kode dan prefix berbeda
- **Multi-loket** — setiap loket dapat dipanggil secara independen
- **Cetak tiket** — integrasi printer thermal lokal maupun remote (print agent)
- **Display publik** — layar antrian dengan riwayat panggilan dan status loket
- **Panel admin** — manajemen antrian, loket, pengaturan tampilan, dan laporan
- **Laporan & ekspor** — statistik harian dan ekspor CSV

---

## Tech Stack

| Komponen | Teknologi |
|---|---|
| Backend | Go (net/http, embed) |
| Database | SQLite (via modernc/sqlite — tanpa CGO) |
| Frontend | HTML, CSS, JavaScript (vanilla) |
| Real-time | Server-Sent Events (SSE) |
| Konfigurasi | YAML |
| Desktop wrapper | Tauri 2 (opsional) |

---

## Persyaratan Sistem

- **Go** 1.21 atau lebih baru
- **OS**: Windows, Linux, atau macOS
- Tidak memerlukan database server eksternal (SQLite sudah ter-embed)

---

## Setup & Instalasi

### 1. Clone repositori

```bash
git clone <url-repositori>
cd antrian-kpp
```

### 2. Download dependensi

```bash
go mod download
```

### 3. Buat file konfigurasi

Salin contoh konfigurasi dan sesuaikan:

```bash
cp configs/config.example.yaml config.yaml
```

Buka `config.yaml` dan sesuaikan pengaturan:

```yaml
server:
  host: "0.0.0.0"   # 0.0.0.0 agar bisa diakses dari perangkat lain di jaringan
  port: 8080

security:
  admin_password: "ganti_dengan_password_anda"   # akan otomatis di-hash saat pertama kali dijalankan
  session_timeout: 3600   # durasi sesi admin dalam detik

queue:
  reset_daily: true        # reset nomor antrian setiap hari

printer:
  enabled: false           # aktifkan jika ada printer thermal terhubung langsung
  printer_name: "ECO80"
```

> **Catatan keamanan:** `admin_password` di `config.yaml` boleh diisi plaintext. Saat server pertama kali dijalankan, password akan otomatis di-hash menggunakan bcrypt dan plaintext akan dihapus dari file konfigurasi.

### 4. Jalankan server

```bash
# Menggunakan Makefile
make run

# Atau langsung dengan Go
go run . -config config.yaml
```

Server akan berjalan di `http://0.0.0.0:8080`. Browser akan terbuka otomatis menuju halaman admin.

### 5. Build binary (opsional)

```bash
# Build development
make build

# Build production (optimized)
make build-prod

# Build untuk Windows
make build-windows
```

Binary akan tersimpan di folder `bin/`.

---

## Penggunaan

Setelah server berjalan, buka browser dari perangkat manapun yang terhubung ke jaringan yang sama dan akses URL berikut (ganti `IP-SERVER` dengan IP komputer yang menjalankan server, contoh: `192.168.1.10`):

### Halaman-halaman

| Halaman | URL | Deskripsi | Diakses oleh |
|---|---|---|---|
| **Display** | `http://IP-SERVER:8080/display` | Layar antrian publik | Monitor ruang tunggu |
| **Ambil Tiket** | `http://IP-SERVER:8080/ticket` | Form ambil nomor antrian | Wajib pajak / mesin tiket |
| **Pilih Loket** | `http://IP-SERVER:8080/counters` | Daftar loket tersedia | Petugas saat pertama buka |
| **Loket** | `http://IP-SERVER:8080/counter/{id}` | Antarmuka kerja petugas loket | Petugas per loket |
| **Admin** | `http://IP-SERVER:8080/admin` | Panel manajemen sistem | Administrator |

### Alur penggunaan

```
1. Wajib pajak buka /ticket → ambil nomor antrian
2. Monitor display /display → menampilkan nomor yang dipanggil secara real-time
3. Petugas buka /counter/{id} → tekan "Panggil" untuk memanggil nomor berikutnya
4. Admin buka /admin → kelola loket, jenis antrian, pengaturan, dan laporan
```

### Panel Admin

Akses `/admin` memerlukan password. Fitur yang tersedia:

- **Dashboard** — statistik antrian hari ini
- **Kelola Antrian** — lihat dan reset antrian
- **Loket** — tambah, edit, aktifkan/nonaktifkan loket
- **Jenis Antrian** — konfigurasi kode, nama, dan prefix antrian
- **Pengaturan Tampilan** — kustomisasi teks display dan running text
- **Tiket & Cetak** — konfigurasi template tiket
- **Laporan** — statistik per rentang tanggal dan ekspor CSV

---

## Konfigurasi Jaringan

Agar perangkat lain di jaringan dapat mengakses server:

1. **Pastikan IP server statis** — atur IP tetap di pengaturan jaringan Windows agar tidak berubah setiap kali restart
2. **Izinkan port di firewall** — jalankan perintah berikut di PowerShell sebagai Administrator:

```powershell
netsh advfirewall firewall add rule name="Antrian KPP" dir=in action=allow protocol=TCP localport=8080
```

3. **Bookmark URL** di browser masing-masing perangkat agar tidak perlu mengetik ulang setiap hari

---

## Backup Database

Gunakan script yang tersedia untuk backup otomatis database SQLite:

```bash
# Backup ke folder default (./backups)
./scripts/backup.sh

# Backup ke folder tertentu
./scripts/backup.sh /path/ke/folder/backup
```

Script akan membuat file backup bertanggal dan menghapus otomatis backup yang lebih dari 7 hari.

---

## Menjalankan sebagai Service (Linux)

Untuk menjalankan server secara otomatis saat sistem menyala:

```bash
# Salin binary ke direktori instalasi
sudo make install

# Install service systemd
sudo make install-service

# Aktifkan dan jalankan service
sudo systemctl enable queue-system
sudo systemctl start queue-system
```

---

## Aplikasi Desktop (Tauri)

Selain akses via browser, tersedia tiga aplikasi desktop ringan berbasis **Tauri 2** yang dapat diinstal di masing-masing perangkat operasional. Aplikasi ini membungkus halaman web menjadi jendela aplikasi tersendiri — lebih rapi, tidak bisa ditutup tidak sengaja, dan langsung terbuka di halaman yang tepat tanpa harus mengetik URL.

### Daftar aplikasi

| Aplikasi | Folder | Halaman yang dibuka | Digunakan untuk |
|---|---|---|---|
| **Antrian Display** | `tauri-display/` | `/display` | Monitor layar antrian di ruang tunggu |
| **Antrian Loket** | `tauri-counter/` | `/counters` → `/counter/{id}` | Komputer petugas di meja loket |
| **Antrian Tiket** | `tauri-ticket/` | `/ticket` | Mesin atau komputer pengambilan tiket |

### Cara build aplikasi Tauri

Pastikan sudah menginstal [Rust](https://rustup.rs) dan [Node.js](https://nodejs.org), kemudian:

```bash
# Masuk ke folder aplikasi yang ingin di-build, contoh untuk Display:
cd tauri-display

# Install dependensi Node
npm install

# Jalankan dalam mode development
npm run tauri dev

# Build installer (.exe untuk Windows)
npm run tauri build
```

Installer hasil build tersedia di `src-tauri/target/release/bundle/`.

> **Catatan:** Aplikasi Tauri tetap memerlukan server Go yang sedang berjalan. Pastikan server sudah aktif sebelum membuka aplikasi desktop.
