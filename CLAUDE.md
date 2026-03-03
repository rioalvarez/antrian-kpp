# Sistem Antrian KPP — Project Context for Claude Code

## Overview
Sistem antrian digital untuk KPP Pratama. Go backend (port 8080) melayani web UI untuk manajemen antrian dengan real-time updates via SSE. Deploy sebagai **single binary** (semua assets ter-embed). Tiga Tauri 2 desktop app sebagai wrapper client.

## Tech Stack
- **Language:** Go (module: `queue-system`, Go 1.25+)
- **Database:** SQLite via `modernc.org/sqlite` (pure Go, tanpa CGO)
- **Real-time:** Server-Sent Events (SSE)
- **Config:** YAML (`config.yaml`, auto-created jika tidak ada)
- **Desktop wrappers:** Tauri 2 (tauri-display, tauri-counter, tauri-ticket)
- **Thermal printer:** ESC/POS via PowerShell + Windows Spooler

---

## Project Structure
```
antrian-kpp/
├── main.go                          # Entry point: HTTP server, embed, background tasks
├── config.yaml                      # Config aktif (auto-generated)
├── go.mod / go.sum
├── Makefile
├── data/queue.db                    # SQLite (auto-created)
├── internal/
│   ├── config/config.go             # Config struct, load/save, bcrypt hash password
│   ├── models/models.go             # Semua data models & SSE event structs
│   ├── database/database.go         # DB layer: schema migration + semua CRUD
│   ├── handlers/handlers.go         # Semua HTTP handlers & route registration (~1400 lines)
│   ├── sse/hub.go                   # SSE hub: client management & broadcasting
│   └── printer/printer.go           # ESC/POS commands + PowerShell print
├── cmd/print-agent/
│   ├── main.go                      # Entry point remote print agent
│   ├── agent.go                     # SSE listener + job processor
│   └── config.go                    # AgentConfig struct
├── web/
│   ├── templates/                   # HTML templates (di-embed ke binary)
│   │   ├── display.html             # Layar display publik
│   │   ├── counter.html             # Interface operator loket
│   │   ├── counters.html            # Halaman pilih loket
│   │   ├── admin.html               # Panel admin (SPA)
│   │   ├── admin_login.html
│   │   ├── ticket.html              # Kiosk ambil nomor antrian
│   │   └── super_counter.html       # Multi-loket (supervisor, require auth)
│   └── static/
│       ├── css/                     # Stylesheet per halaman
│       ├── js/                      # Logic per halaman
│       │   ├── display.js           # Display + audio system (~1200 lines)
│       │   ├── admin.js             # Panel admin SPA (~1500 lines)
│       │   ├── counter.js           # Operator loket
│       │   └── super-counter.js     # Supervisor multi-loket
│       └── audio/
│           ├── perempuan/*.mp3      # Voice pack perempuan
│           └── laki-laki/*.mp3      # Voice pack laki-laki
├── tauri-display/                   # Tauri wrapper: monitor display
├── tauri-counter/                   # Tauri wrapper: operator loket
└── tauri-ticket/                    # Tauri wrapper: kiosk
```

---

## Web Interfaces & Routes (Pages)
| URL | Pengguna | Keterangan |
|-----|---------|------------|
| `GET /` | — | Redirect ke `/display` |
| `GET /display` | Monitor publik | Display antrian real-time (pass `AudioEnabled`, `BellFile`) |
| `GET /ticket` | Kiosk / pelanggan | Ambil nomor antrian (pass `QueueTypes` aktif) |
| `GET /counters` | Petugas | Pilih nomor loket |
| `GET /counter/{id}` | Petugas | Interface panggil antrian (pass `Counter` + `QueueTypes`) |
| `GET /admin` | Admin | Panel admin SPA — **require auth** |
| `GET /admin/super-counter` | Supervisor | Semua loket satu layar — **require auth** |
| `GET /admin/login` / `POST /admin/login` | Admin | Form login |
| `GET /admin/logout` | Admin | Clear session |
| `GET /health` | — | JSON: status, timestamp, display_clients, stats |

---

## API Routes Lengkap

### Queues
| Route | Method | Keterangan |
|-------|--------|------------|
| `/api/queues` | GET | Params: `status`, `type`, `date`, `page`, `per_page` (default 20, max 100) |
| `/api/queues/take` | POST | `?type={code}` — buat antrian baru. Broadcast `queue_added` ke semua counter |

### Counters
| Route | Method | Keterangan |
|-------|--------|------------|
| `/api/counters` | GET/POST | List semua / create baru |
| `/api/counter/{id}` | GET/PUT/DELETE | CRUD individual |
| `/api/counter/{id}/call-next` | POST | `?type={code}` — atomik: complete current → call next |
| `/api/counter/{id}/recall` | POST | Panggil ulang antrian saat ini |
| `/api/counter/{id}/complete` | POST | Selesaikan antrian saat ini |
| `/api/counter/{id}/cancel` | POST | Skip/batalkan antrian saat ini |

### Queue Types
| Route | Method | Keterangan |
|-------|--------|------------|
| `/api/queue-types` | GET | `?active=true` untuk filter aktif saja |
| `/api/queue-types` | POST | Create baru |
| `/api/queue-type/{id}` | GET/PUT/DELETE | CRUD individual |

### Stats & Settings
| Route | Method | Keterangan |
|-------|--------|------------|
| `/api/stats` | GET | Total/waiting/called/completed/cancelled/active_counters hari ini |
| `/api/stats/by-type` | GET | `map[queueTypeCode]waitingCount` hari ini |
| `/api/settings` | GET | `?keys=k1,k2` untuk specific keys, atau semua jika kosong |
| `/api/settings` | POST | Upsert `map[string]string`. Juga broadcast `settings_updated` ke display |
| `/api/audio-voices` | GET | List subdirektori di `web/static/audio/` (voice packs) |
| `/api/admin/reset-queues` | POST | **Require auth.** `?type=A` (kosong = semua). Broadcast `queue_reset` |

### Reports
| Route | Keterangan |
|-------|------------|
| `GET /api/report?start=YYYY-MM-DD&end=YYYY-MM-DD` | Statistik + breakdown harian + by-type |
| `GET /api/report/export?start=...&end=...` | Download CSV |

### Printer
| Route | Keterangan |
|-------|------------|
| `POST /api/print-ticket` | Cetak tiket (local + remote sekaligus) |
| `POST /api/printer/test` | Test print |
| `GET /api/printer/status` | `{enabled, printer_name, remote_enabled, agents_online}` |

### Print Agent (Remote)
| Route | Keterangan |
|-------|------------|
| `GET /api/print-agent/sse?agent_id={id}` | SSE endpoint untuk agent; event `print_job` |
| `GET /api/print-agent/jobs/pending` | List semua pending jobs |
| `GET /api/print-agent/job/{id}` | Detail job |
| `POST /api/print-agent/job/{id}/claim` | Atomic claim (409 jika sudah diklaim) |
| `POST /api/print-agent/job/{id}/complete` | Mark completed |
| `POST /api/print-agent/job/{id}/fail` | Body: `{error}`. Mark failed |

### SSE
| Route | Keterangan |
|-------|------------|
| `GET /api/sse/display` | SSE untuk halaman display |
| `GET /api/sse/counter/{id}` | SSE untuk counter tertentu |

---

## Database Schema (SQLite)
```sql
queues       (id, queue_number, queue_type, status, counter_id, created_at, called_at, completed_at)
counters     (id, counter_number, counter_name, is_active, current_queue_id, last_call_at)
queue_types  (id, code UNIQUE, name, prefix, is_active, sort_order, created_at)
settings     (key TEXT PK, value, updated_at)
call_history (id, queue_id, counter_id, action, timestamp)
print_jobs   (id, queue_number, type_name, date_time, template_json, status,
              agent_id, created_at, claimed_at, completed_at, error_message)
```

**PENTING:** Semua query antrian selalu difilter `DATE(created_at) = DATE('now', 'localtime')` — data per hari otomatis terpisah.

**Counter current_queue:** `GetCounter()` hanya mengembalikan `CurrentQueue` jika `called_at` hari ini — antrian kemarin di-ignore otomatis meski `current_queue_id` masih terisi.

**Default seeding:** Jika DB kosong, satu queue type otomatis dibuat: `code=A, name=Umum, prefix=A`.

---

## Key Database Operations

### `CreateQueue(queueTypeCode)` — transaksi
Cari prefix dari queue_type → hitung nomor terakhir hari ini → buat `{prefix}{N:03d}`

### `CallNextQueue(counterID, queueType)` — transaksi atomik
1. Jika counter punya current queue → mark `completed`, insert call_history
2. Cari antrian `waiting` tertua hari ini dengan tipe yang diminta
3. Update queue → `status=called`, `counter_id`, `called_at=now`
4. Update counter → `current_queue_id`, `last_call_at=now`
5. Insert call_history (called)

### `ResetQueuesToday(queueType)` — transaksi
Collect queue IDs hari ini → reset `current_queue_id` counter terkait → hapus call_history → hapus queues

---

## SSE Hub (`internal/sse/hub.go`)

### Tipe Client
```go
ClientTypeDisplay (0)  // → displayClients map
ClientTypeCounter (1)  // → counterClients[counterID] map
ClientTypePrinter (2)  // → printerClients map
```

**KRITIS:** `ClientType` **wajib diset dengan benar** saat membuat Client di `serveSSE()`. Kalau tidak di-set (default 0 = Display), counter client tidak akan pernah menerima event dari `BroadcastCounter`/`BroadcastAllCounters`.

### SSE Event Types

| Event | Target | Trigger |
|-------|--------|---------|
| `queue_called` | Display | call-next, recall |
| `queue_added` | All Counters | take queue |
| `queue_updated` | All Counters | call-next, complete, cancel |
| `queue_reset` | All Counters | reset-queues |
| `settings_updated` | Display | POST /api/settings |
| `print_job` | Printers | POST /api/print-ticket (jika remote_enabled) |

- Heartbeat comment (`: heartbeat`) setiap 30 detik
- Buffer channel 100 message per client
- `WriteTimeout` server di-set 0 agar SSE tidak terputus

---

## Auth Flow

1. `POST /admin/login` → verifikasi bcrypt → generate 32-byte random token (hex)
2. Token disimpan di in-memory `map[token]expiry`; cookie `admin_session` (HttpOnly, SameSite=Strict)
3. Session timeout: `security.session_timeout` detik (default 3600)
4. **In-memory only** — restart server invalidate semua session
5. Routes yang diproteksi: `/admin`, `/admin/super-counter`, `/api/admin/reset-queues`

---

## Config Structure (`config.yaml`)
```yaml
server:    { host: 0.0.0.0, port: 8080, read_timeout, write_timeout }
database:  { path: ./data/queue.db, max_open_conns, max_idle_conns }
logging:   { level, output, file_path }
queue:     { prefix: A, start_number: 1, reset_daily: true, auto_cancel_hours: 24 }
audio:     { enabled, bell_file }
security:  { admin_password: admin123, session_timeout: 3600 }
printer:   { enabled: true, printer_name: ECO80, remote_enabled: false }
```

**Password auto-hash:** Saat startup, jika `admin_password` bukan bcrypt hash, otomatis di-hash dan disimpan kembali ke config.yaml.

---

## Handler Struct
```go
type Handler struct {
    db         *database.DB
    hub        *sse.Hub
    config     *config.Config
    tmpl       *template.Template
    staticFS   fs.FS           // fs.Sub(webFS, "web/static") — melayani /static/*
    printer    *printer.Printer
    sessions   map[string]time.Time
    sessionsMu sync.RWMutex
}
```

`staticFS` adalah sub-FS dari embedded `webFS`. `handleAudioVoices` membaca subdirektori dari `staticFS` path `"audio"` (= `web/static/audio/`).

**CORS:** `jsonResponse` dan `jsonError` selalu menyertakan `Access-Control-Allow-Origin: *` — diperlukan untuk fetch dari Tauri webview ke Go server.

---

## Audio TTS System (`web/static/js/display.js`)

### Tiga Mode (setting `display_tts_mode` di DB):

| Mode | Cara kerja |
|------|-----------|
| `local_tts` | Tauri inject `window.USE_LOCAL_TTS=true`; audio dari `local-audio://localhost/{file}` (flat dir, tanpa subfolder suara). Default. |
| `server_audio` | Browser fetch `/static/audio/{AUDIO_VOICE}/{file}`. `AUDIO_VOICE` = nilai `display_audio_voice` dari DB. |
| `web_speech` | Browser Speech Synthesis API. Gunakan template `display_tts_template`. |

**`display_audio_voice`** (default: `perempuan`) hanya disimpan ke DB saat mode adalah `server_audio` (fix di `admin.js saveDisplaySound()`). Saat mode lain, voice selector tersembunyi dan nilainya tidak disimpan.

### File Audio yang Dibutuhkan (per voice pack, di subfolder)
```
nomor_antrian.mp3, silakan_menuju.mp3, loket.mp3
huruf_a.mp3 ... huruf_z.mp3
angka_0.mp3 ... angka_9.mp3, angka_10.mp3 ... angka_19.mp3
angka_20.mp3, angka_30.mp3, ... angka_90.mp3
puluh.mp3, ratus.mp3, ribu.mp3, seratus.mp3, seribu.mp3
```

### Audio Queue System
```
queueAudio(number, counter, type) → audioQueue[]
processAudioQueue() → bell (500ms) → announceQueue() → 300ms gap → next
```
Audio dimainkan **sekuensial** (tidak overlap). `audioQueue[]` di-buffer jika ada antrian dipanggil cepat berturutan.

### Urutan Eksekusi Audio di Tauri Display
1. Halaman load → `display.js` define `announceQueue()`
2. `on_page_load` fires → `webview.eval()` inject `main-injection.js` + `local-tts.js`
3. `local-tts.js` override `window.announceQueue` (simpan original sebagai `originalAnnounceQueue`)
4. Settings load dari `/api/settings` → `updateDisplaySettings()` set `TTS_MODE` & `USE_LOCAL_TTS`
5. Saat queue dipanggil: Tauri's `window.announceQueue` cek `USE_LOCAL_TTS`:
   - `true` → `announceQueueLocal()` (local MP3 flat dir)
   - `false` → `originalAnnounceQueue()` → cek `TTS_MODE` → `announceQueueServerAudio()` atau web_speech

### Polling Fallback (display.js)
SSE terputus → fallback polling setiap 2 detik via `GET /api/queues?status=called&limit=1`. Bandingkan dengan `lastQueueCalled`.

---

## Display Frontend Global State (`display.js`)

| Variable | Tipe | Keterangan |
|----------|------|------------|
| `recentCalls` | Array | {queue_number, counter_name, counter_id, queue_type, timestamp}. Max `MAX_RECENT_CALLS` (6) |
| `callHistory` | Array | {queue_number, counter_name, timestamp}. Max `MAX_HISTORY` (15) |
| `counterStatus` | Object | Map counterID → {queue_number, queue_type, called_at} |
| `TTS_MODE` | string | `local_tts` / `server_audio` / `web_speech` |
| `AUDIO_VOICE` | string | Subfolder voice pack, default `perempuan` |
| `audioQueue` | Array | Queue audio sequential yang menunggu |
| `isPlayingAudio` | bool | Flag sedang memutar audio |
| `displaySettings` | Object | Semua settings dari DB |

### Display Settings Keys Penting
| Key | Default | Keterangan |
|-----|---------|------------|
| `display_tts_mode` | `local_tts` | Mode audio |
| `display_audio_voice` | `perempuan` | Voice pack subdirectory |
| `display_tts_template` | `Nomor antrian {nomor}, silakan menuju {loket}` | Template TTS |
| `display_tts_rate` | `0.6` | Kecepatan TTS |
| `display_ticker_speed` | `45` | Detik animasi ticker |
| `display_recent_calls_count` | `6` | Jumlah kartu recent calls |
| `display_history_count` | `15` | Jumlah item history |
| `display_show_video` | `true` | Toggle video player |
| `display_show_stats` | `true` | Toggle stats header |
| `display_sound_enabled` | `true` | Toggle audio keseluruhan |

**Pattern boolean settings:** `settings.xxx !== 'false'` → default `true` jika key belum ada di DB (value kosong/tidak ada).

---

## Admin Frontend (`admin.js`)

**SPA dengan 6 halaman:**
| Page ID | Isi |
|---------|-----|
| `page-dashboard` | Stats cards + counter cards |
| `page-management` | Tabel antrian + filter (status, tipe, tanggal) |
| `page-display-settings` | Pengaturan tampilan + audio display |
| `page-ticket-settings` | Desain tiket + tampilan halaman ticket |
| `page-system-settings` | Jam operasional + batas antrian |
| `page-reports` | Laporan harian + export CSV |

- `localStorage.setItem('admin_current_page', ...)` — restore halaman terakhir
- `loadDisplayAppearanceSettings()` memanggil `onTtsModeChange()` lalu `loadAudioVoices()` — urutan ini penting

---

## Counter Frontend (`counter.js`)

- `COUNTER_ID` dan `COUNTER_NAME` di-inject dari Go template (`counter.html`)
- `counter.html` hanya menampilkan `counter_name` di header (tidak ada `counter_number`)
- Polling fallback jika SSE putus: `loadCounterData()` + `loadStatsByType()` setiap 3 detik
- SSE events yang ditangani: `queue_updated`, `queue_added`

---

## Super Counter (`super-counter.js` + `super_counter.html`)

Operator bisa mengoperasikan **semua loket** dari satu halaman. Navigasi 3-state:
```
State 1: Pilih Jenis Antrian (badge waiting count)
  → State 2: Pilih Loket (kartu per loket + status dot)
    → State 3: Panel Aksi (Panggil, Recall, Selesai, Lewati)
```
Data di-inject via Go template: `const QUEUE_TYPES = [...]` dan `const ALL_COUNTERS = [...]`.
Polling setiap 3 detik: update badge waiting count + update nomor antrian tiap loket.

---

## Printer System

### ESC/POS Layout Tiket
```
[CENTER+BOLD]    Header
[FONT_B]         Subheader
────────────────────────────
[FONT_B]         Title
[DOUBLE+BOLD]    QueueNumber  ← besar
[BOLD]           TypeName
────────────────────────────
[FONT_B]         DateTime
────────────────────────────
[FONT_B]         Footer1, Footer2, Thanks
[FEED 3][CUT]
```

### Mekanisme Cetak (Windows)
1. Tulis bytes ke temp file
2. PowerShell script: coba WMI → coba serial port → fallback inline C# (`winspool.Drv`)

### Remote Print Agent Flow
```
Start → catchUpPendingJobs() [REST: GET /api/print-agent/jobs/pending]
      → subscribeSSE() [SSE: /api/print-agent/sse?agent_id=xxx]
          event print_job → goroutine processJob(id):
            1. POST /api/print-agent/job/{id}/claim  ← atomic
            2. Parse template_json → TicketTemplate
            3. printer.PrintTicket() via PowerShell
            4. POST complete / fail
```

**Anti-double-print:** `ClaimPrintJob` menggunakan `UPDATE WHERE status='pending'` + cek `RowsAffected`. Jika sudah diklaim agent lain → return error, job di-skip.

---

## Tauri Apps

### tauri-counter (`tauri-counter/src-tauri/src/main.rs`)

**Config `config.json`** (di samping .exe):
```json
{ "serverUrl": "http://192.168.x.x:8080", "counterId": 3, "counterName": "Loket A1" }
```

**Commands:** `get_server_config`, `save_config(server_url, counter_id, counter_name)`, `navigate_to_counter(id)`, `navigate_to_counters()`

**Auto-save banner:** Saat navigasi ke `/counter/{id}`, setelah 1 detik inject banner JS "Simpan sebagai Default?" di pojok kanan bawah. Banner fetch nama dari `/api/counter/{id}`, invoke `save_config` saat klik. Auto-dismiss 15 detik. `parse_counter_id_from_url()` skip URL yang mengandung `/api/counter/`.

**Startup flow:** Baca `config.json` → jika ada `counterId` → navigasi ke `/counter/{id}` langsung.

**Window size:** 900×700, minimum 480×600.

### tauri-display (`tauri-display/src-tauri/src/main.rs`)

**Config `config.json`:**
```json
{
  "serverUrl": "http://192.168.x.x:8080",
  "displayPath": "/display",
  "fullscreen": true,
  "kiosk": true,
  "devTools": false,
  "useLocalTts": true
}
```

**Commands:** `get_server_config`, `save_server_url(url)`, `navigate_to_server`

**Custom URI Protocol `local-audio://`:**
- Resolve: `resource_dir/audio/{filename}` → `exe_dir/audio/{filename}` → `cwd/audio/{filename}`
- **Flat directory** — tidak ada subfolder suara. Admin voice selector tidak berpengaruh ke mode ini.
- Windows: `http://local-audio.localhost/{file}`, non-Windows: `local-audio://localhost/{file}`

**JS injection via `on_page_load`** (hanya untuk URL server, skip `tauri://localhost`):
- Eval `main-injection.js` + (jika `use_local_tts=true`) `window.USE_LOCAL_TTS=true` + `local-tts.js`
- 1 detik kemudian: inject `audioEnabled=true`, remove tombol "Aktifkan Audio"

**Global Shortcuts:**
| Shortcut | Aksi |
|----------|------|
| F5 | Reload dari server |
| F11 | Toggle fullscreen |
| Ctrl+Shift+D | Toggle DevTools |
| Ctrl+Shift+L | Test Local TTS |
| Ctrl+Shift+T | Test Web Speech TTS |
| Ctrl+Q | Exit |

---

## Background Tasks (main.go)

Setiap 1 jam:
- `db.CancelOldQueues(cfg.Queue.AutoCancelHours)` — auto-cancel antrian waiting > X jam
- `db.CleanupOldPrintJobs(24)` — hapus print jobs completed/failed > 24 jam

Graceful shutdown: timeout 10 detik saat `SIGINT`/`SIGTERM`.
Auto-open browser ke `/admin` saat startup (delay 1 detik).

---

## Common Commands

```bash
# Run
go run .                         # jalankan dengan config.yaml di cwd
make run                         # sama, via Makefile

# Build server
make build                       # development
make build-prod                  # -ldflags="-s -w" (optimized)
go build -ldflags="-s -w" -o release/server/antrian-kpp.exe .

# Build print agent
go build -o print-agent.exe ./cmd/print-agent/

# Build Tauri
cd tauri-counter && npm run tauri build
cd tauri-display && npm run tauri build

# Build release package (interactive .bat)
./build-release.bat              # menghasilkan release/server/
```

---

## Known Patterns & Gotchas

1. **Date filtering:** Semua query antrian filter `DATE(created_at) = DATE('now', 'localtime')` — data hari sebelumnya tidak muncul di operasional normal.

2. **Nomor antrian format:** `{prefix}{N:03d}` (A001, B012). Nomor per-prefix, reset per hari.

3. **Settings boolean:** Disimpan sebagai string `"true"/"false"`. Cek: `settings.xxx !== 'false'` (default true jika key belum ada).

4. **SSE WriteTimeout=0:** `main.go` sengaja set 0 agar SSE tidak terputus oleh HTTP timeout.

5. **SSE ClientType wajib:** Kalau `ClientType` tidak di-set di `serveSSE()`, default ke 0 (Display) → counter client tidak terima broadcast counter. Bug ini pernah terjadi dan sudah diperbaiki.

6. **Print job atomic claim:** `UPDATE WHERE status='pending'` + cek `RowsAffected==0` → anti-double-print jika beberapa agent online.

7. **In-memory sessions:** Restart server = logout semua admin session.

8. **Embedded assets:** Semua file `web/` di-embed via `//go:embed all:web`. Deploy = 1 binary saja.

9. **Audio voice selector:** Hanya disimpan ke DB saat mode `server_audio` aktif (hidden state tidak di-save).

10. **Counter current_queue JOIN filter:** JOIN di `GetCounter` filter `DATE(q.called_at) = today` — antrian kemarin otomatis tidak muncul di UI counter meski ID masih tersimpan.

11. **build-release.bat:** Menghasilkan `release/server/` (binary tunggal). File audio di `web/static/audio/` opsional — hanya diperlukan untuk mode `server_audio`. Mode `local_tts` menggunakan file dari `audio/` di samping .exe Tauri.

---

## Dependencies Go
```
modernc.org/sqlite       # SQLite pure Go
gopkg.in/yaml.v3         # YAML config
golang.org/x/crypto      # bcrypt
github.com/google/uuid   # UUID
```
