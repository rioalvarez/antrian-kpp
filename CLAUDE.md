# Sistem Antrian KPP - Project Context for Claude Code

## Overview
Sistem antrian digital untuk KPP Pratama. Go backend (port 8080) melayani web UI untuk manajemen antrian dengan real-time updates via SSE. Bisa di-deploy sebagai binary tunggal.

## Tech Stack
- **Language**: Go 1.25+
- **Database**: SQLite (modernc.org/sqlite, no CGO needed)
- **Real-time**: Server-Sent Events (SSE)
- **Config**: YAML (`config.yaml` di root)
- **Desktop wrappers**: Tauri 2 (lighter than Electron)
- **Thermal printer**: ESC/POS via Windows Print Spooler

## Project Structure
```
antrian-kpp/
├── main.go                    # Entry point, HTTP server, embed assets
├── config.yaml                # Main config (auto-created if missing)
├── go.mod / go.sum
├── Makefile
├── internal/
│   ├── config/config.go       # Config struct, bcrypt password hashing
│   ├── database/database.go   # SQLite DB, schema migration, all DB ops
│   ├── handlers/handlers.go   # All HTTP handlers & route registration (~1300 lines)
│   ├── models/models.go       # Data models (Queue, Counter, QueueType, etc.)
│   ├── sse/hub.go             # SSE hub, client management, broadcasting
│   └── printer/printer.go     # ESC/POS thermal printer commands
├── cmd/
│   └── print-agent/           # Remote print agent (runs on printer PC)
│       ├── main.go
│       ├── agent.go           # SSE client, job claiming, local print
│       ├── config.go
│       └── config.yaml
├── web/
│   ├── templates/             # HTML templates (embedded into binary)
│   │   ├── display.html       # Public queue display screen
│   │   ├── counter.html       # Petugas loket interface
│   │   ├── counters.html      # Pilih nomor loket
│   │   ├── admin.html         # Admin dashboard
│   │   ├── admin_login.html
│   │   ├── ticket.html        # Ambil nomor antrian (kiosk)
│   │   └── super_counter.html # Tampilan semua loket (supervisor)
│   └── static/
│       ├── css/               # Per-page stylesheets
│       ├── js/                # Per-page JS (display.js, admin.js, counter.js, super-counter.js)
│       └── audio/             # TTS audio files (huruf_*.mp3, angka_*.mp3, laki-laki/, perempuan/)
├── tauri-display/             # Tauri 2 wrapper untuk monitor display
├── tauri-counter/             # Tauri 2 wrapper untuk loket petugas
└── tauri-ticket/              # Tauri 2 wrapper untuk mesin kiosk
```

## Web Interfaces & Routes
| URL | Pengguna | Keterangan |
|-----|---------|------------|
| `/display` | Monitor publik | Tampilan antrian real-time |
| `/ticket` | Kiosk / pelanggan | Ambil nomor antrian |
| `/counters` | Petugas | Pilih nomor loket |
| `/counter/{id}` | Petugas | Antarmuka panggil antrian |
| `/admin` | Admin | Dashboard, manajemen, laporan |
| `/admin/super-counter` | Supervisor | Semua loket dalam satu layar |

## Database Schema (SQLite)
- `queues` — nomor antrian, status (waiting/called/completed/cancelled), counter assignment
- `counters` — definisi loket, current queue
- `queue_types` — jenis layanan (Prefix A/B/C, nama, urutan)
- `settings` — konfigurasi UI sebagai key-value store
- `call_history` — audit log semua aksi (called/recalled/completed/cancelled)
- `print_jobs` — antrian cetak untuk remote print agent

## Key Architecture Patterns
- **All assets embedded** ke binary via `go:embed` — deploy cukup 1 file
- **SSE hub** di `internal/sse/hub.go`: display clients, counter clients (per-ID), printer clients
- **Real-time flow**: Counter panggil → handler → broadcast SSE → display & counter update
- **Session auth**: token 64 hex chars, in-memory map, HttpOnly cookie, timeout 1 jam
- **Remote printing**: print job disimpan di DB → broadcast SSE ke print agent → agent claim & cetak lokal
- **Password**: bcrypt, auto-hash dari plaintext di config pada first run

## Handler Structure (`internal/handlers/handlers.go`)
```go
type Handler struct {
    db       *database.DB
    hub      *sse.Hub
    config   *config.Config
    tmpl     *template.Template
    staticFS fs.FS
    printer  *printer.Printer
    sessions map[string]time.Time
    sessionsMu sync.RWMutex
}
```

## Config Structure (`config.yaml`)
```yaml
server:    { host, port: 8080, read_timeout, write_timeout }
database:  { path: ./data/queue.db, max_open_conns, max_idle_conns }
logging:   { level, output, file_path }
queue:     { prefix, start_number, reset_daily, auto_cancel_hours: 24 }
audio:     { enabled, bell_file }
security:  { admin_password, session_timeout: 3600 }
printer:   { enabled, printer_name: ECO80, remote_enabled }
```

## Common Tasks

### Run server
```bash
go run main.go
# atau
make run
```

### Build
```bash
make build           # development
make build-prod      # production (optimized)
make build-windows   # Windows binary
```

### Build Tauri app
```bash
cd tauri-counter/
npm install
npm run tauri dev    # test
npm run tauri build  # create installer (.exe)
```

### Build print agent
```bash
go build -o print-agent.exe ./cmd/print-agent/
```

## Dependencies
```
gopkg.in/yaml.v3         # YAML config
modernc.org/sqlite       # SQLite (pure Go, no CGO)
golang.org/x/crypto      # bcrypt
github.com/google/uuid   # UUID generation
```

## Development Notes
- Semua perubahan template/static langsung efektif setelah restart (embedded saat build)
- Untuk development, file di `web/` dibaca dari disk, bukan dari embed
- SQLite WAL mode aktif — aman untuk concurrent read
- Cleanup otomatis: auto-cancel antrian lama & hapus print job lama setiap jam
- Browser auto-open ke `/admin` saat server start
- `super_counter.html` dan `super-counter.js/css` adalah fitur baru (belum di-commit)
- Audio TTS: file MP3 per kata (huruf & angka) di `web/static/audio/`, ada subfolder per suara (laki-laki/perempuan)
