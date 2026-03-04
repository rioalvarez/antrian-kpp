# Audio Files untuk TTS Lokal

Folder ini berisi file audio untuk pengumuman antrian.

## Struktur File yang Diperlukan

### Kata-kata Utama
- `nomor_antrian.mp3` - "Nomor antrian"
- `silakan_menuju.mp3` - "silakan menuju"
- `loket.mp3` - "Loket"

### Huruf (untuk prefix antrian)
- `huruf_a.mp3` - "A"
- `huruf_b.mp3` - "B"
- `huruf_c.mp3` - "C"
- ... (sesuai prefix yang digunakan)

### Angka Dasar (0-11)
- `angka_0.mp3` - "nol"
- `angka_1.mp3` - "satu"
- `angka_2.mp3` - "dua"
- `angka_3.mp3` - "tiga"
- `angka_4.mp3` - "empat"
- `angka_5.mp3` - "lima"
- `angka_6.mp3` - "enam"
- `angka_7.mp3` - "tujuh"
- `angka_8.mp3` - "delapan"
- `angka_9.mp3` - "sembilan"
- `angka_10.mp3` - "sepuluh"
- `angka_11.mp3` - "sebelas"

### Angka Belasan (12-19)
- `angka_12.mp3` - "dua belas"
- `angka_13.mp3` - "tiga belas"
- ... dst

### Puluhan
- `puluh.mp3` - "puluh"
- `angka_20.mp3` - "dua puluh" (opsional, bisa digabung)
- `angka_30.mp3` - "tiga puluh" (opsional)
- ... dst

### Ratusan
- `seratus.mp3` - "seratus"
- `ratus.mp3` - "ratus"

### Ribuan (jika diperlukan)
- `seribu.mp3` - "seribu"
- `ribu.mp3` - "ribu"

### Bell/Ding
- `bell.mp3` - Suara bell sebelum pengumuman

## Cara Merekam Audio Sendiri

### Opsi 1: Rekam Manual
1. Gunakan aplikasi perekam suara (Audacity, dll)
2. Rekam setiap kata/angka secara terpisah
3. Export sebagai MP3 dengan bitrate 128kbps
4. Simpan dengan nama file sesuai format di atas

### Opsi 2: Generate dari Google TTS
Jalankan script `generate-audio.bat` untuk download audio dari Google TTS.

### Opsi 3: Download dari Internet
Cari file audio TTS Indonesia dan rename sesuai format.

## Tips Kualitas Audio
- Sample rate: 44100 Hz
- Bitrate: 128-192 kbps
- Format: MP3
- Durasi per file: 0.5 - 2 detik
- Volume: Konsisten antar file
- Tidak ada noise/hiss
