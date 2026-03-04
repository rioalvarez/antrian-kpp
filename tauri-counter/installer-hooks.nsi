; ==============================================================
; installer-hooks.nsi — Antrian Loket
;
; Input IP via PowerShell InputBox (menggantikan nsDialogs
; yang tidak bisa membaca nilai input secara reliable).
;
; Alur: Welcome → Pilih Folder → Installing (muncul dialog IP) → Selesai
; ==============================================================

Section -WriteServerConfig
  CreateDirectory "$INSTDIR"

  ; Tulis PowerShell script ke file sementara
  StrCpy $R0 "$TEMP\antrian-loket-setup.ps1"
  FileOpen $R1 "$R0" w
  FileWrite $R1 "Add-Type -AssemblyName 'Microsoft.VisualBasic'$\r$\n"
  FileWrite $R1 "$$ip = [Microsoft.VisualBasic.Interaction]::InputBox($\r$\n"
  FileWrite $R1 "    'Masukkan IP Address server antrian.' + [char]13+[char]10 + '(Port 8080 ditambahkan otomatis)' + [char]13+[char]10+[char]13+[char]10 + 'Contoh: 10.9.1.221',$\r$\n"
  FileWrite $R1 "    'Konfigurasi Server - Antrian Loket',$\r$\n"
  FileWrite $R1 "    '10.9.1.221')$\r$\n"
  FileWrite $R1 "if ([string]::IsNullOrWhiteSpace($$ip)) { $$ip = '10.9.1.221' }$\r$\n"
  FileWrite $R1 "[System.IO.File]::WriteAllText('$TEMP\antrian-loket-ip.txt', $$ip.Trim(), [System.Text.Encoding]::UTF8)$\r$\n"
  FileClose $R1

  ExecWait "powershell.exe -ExecutionPolicy Bypass -File $\"$R0$\""
  Delete "$R0"

  ; Baca IP dari file temp, fallback ke default jika gagal
  StrCpy $R9 "10.9.1.221"
  ClearErrors
  FileOpen $R1 "$TEMP\antrian-loket-ip.txt" r
  ${IfNot} ${Errors}
    FileRead $R1 $R9
    FileClose $R1
    Delete "$TEMP\antrian-loket-ip.txt"
  ${EndIf}

  StrLen $R8 $R9
  ${If} $R8 == 0
    StrCpy $R9 "10.9.1.221"
  ${EndIf}

  ; Tulis config.json
  FileOpen $0 "$INSTDIR\config.json" w
  FileWrite $0 '{$\r$\n'
  FileWrite $0 '  "serverUrl": "http://$R9:8080",$\r$\n'
  FileWrite $0 '  "counterPath": "/counters"$\r$\n'
  FileWrite $0 '}$\r$\n'
  FileClose $0

  DetailPrint "config.json ditulis: http://$R9:8080"
SectionEnd

!macro customInstall
!macroend

!macro customUnInstall
!macroend
