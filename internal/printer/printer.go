package printer

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// ESC/POS Commands
var (
	ESC          = byte(0x1B)
	GS           = byte(0x1D)
	INIT         = []byte{ESC, '@'}       // Initialize printer
	ALIGN_CENTER = []byte{ESC, 'a', 1}    // Center alignment
	ALIGN_LEFT   = []byte{ESC, 'a', 0}    // Left alignment
	BOLD_ON      = []byte{ESC, 'E', 1}    // Bold on
	BOLD_OFF     = []byte{ESC, 'E', 0}    // Bold off
	SIZE_6X      = []byte{GS, '!', 0x55}  // 6× width & height (80mm queue number)
	SIZE_4X      = []byte{GS, '!', 0x33}  // 4× width & height (58mm queue number)
	SIZE_NORMAL  = []byte{GS, '!', 0x00}  // Normal size
	FONT_B       = []byte{ESC, 'M', 1}    // Small font
	FONT_A       = []byte{ESC, 'M', 0}    // Normal font
	FEED_LINE    = []byte{ESC, 'd', 1}    // Feed 1 line
)

// Paper size constants
const (
	Paper80mm = "80mm"
	Paper58mm = "58mm"
)

// PrinterConfig holds printer configuration
type PrinterConfig struct {
	PrinterName string
	Enabled     bool
}

// TicketTemplate holds the ticket design template
type TicketTemplate struct {
	Header    string `json:"header"`
	Subheader string `json:"subheader"`
	Title     string `json:"title"`
	Footer1   string `json:"footer1"`
	Footer2   string `json:"footer2"`
	Thanks    string `json:"thanks"`

	ShowSubheader bool `json:"show_subheader"`
	ShowType      bool `json:"show_type"`
	ShowDatetime  bool `json:"show_datetime"`
	ShowFooter    bool `json:"show_footer"`
	ShowThanks    bool `json:"show_thanks"`

	// Paper / hardware settings — set per machine, serialised into print jobs
	// so remote agents receive the value set in admin. Agents may override
	// with their own local config (see cmd/print-agent/agent.go).
	PaperSize string `json:"paper_size"` // "80mm" (default) or "58mm"
	FeedLines int    `json:"feed_lines"` // blank lines before cut (1–3)
}

// DefaultTemplate returns the default ticket template
func DefaultTemplate() TicketTemplate {
	return TicketTemplate{
		Header:        "SISTEM ANTRIAN",
		Subheader:     "",
		Title:         "NOMOR ANTRIAN ANDA",
		Footer1:       "Mohon menunggu hingga",
		Footer2:       "nomor Anda dipanggil",
		Thanks:        "Terima kasih",
		ShowSubheader: true,
		ShowType:      true,
		ShowDatetime:  true,
		ShowFooter:    true,
		ShowThanks:    true,
		PaperSize:     Paper80mm,
		FeedLines:     1,
	}
}

// Printer handles thermal printing
type Printer struct {
	config PrinterConfig
}

// New creates a new printer instance
func New(config PrinterConfig) *Printer {
	return &Printer{config: config}
}

// TicketData holds the data for printing a ticket
type TicketData struct {
	QueueNumber string
	TypeName    string
	DateTime    string
}

// paperLayout returns layout constants derived from paper size.
//
//	charWidth  – printable characters per line (used for separator length)
//	numberSize – ESC/POS command for the large queue-number font
func paperLayout(paperSize string) (charWidth int, numberSize []byte) {
	if paperSize == Paper58mm {
		return 24, SIZE_4X // 4× fits comfortably on 58 mm
	}
	return 32, SIZE_6X // 6× looks great on 80 mm
}

// PrintTicket prints a queue ticket to the thermal printer
func (p *Printer) PrintTicket(data TicketData, tmpl TicketTemplate) error {
	if !p.config.Enabled {
		return fmt.Errorf("printer is disabled")
	}

	// Resolve paper size (fallback to 80 mm)
	paperSize := tmpl.PaperSize
	if paperSize != Paper80mm && paperSize != Paper58mm {
		paperSize = Paper80mm
	}
	charWidth, numberSize := paperLayout(paperSize)
	separator := strings.Repeat("-", charWidth) + "\n"

	// Feed lines before cut — clamp to sensible range
	feedLines := tmpl.FeedLines
	if feedLines < 1 {
		feedLines = 1
	}
	if feedLines > 5 {
		feedLines = 5
	}

	var buf bytes.Buffer
	buf.Write(INIT)

	// ── Header ───────────────────────────────────────────────────────────────
	buf.Write(ALIGN_CENTER)
	buf.Write(BOLD_ON)
	header := tmpl.Header
	if header == "" {
		header = "SISTEM ANTRIAN"
	}
	buf.WriteString(header + "\n")
	buf.Write(BOLD_OFF)

	// Subheader (optional)
	if tmpl.ShowSubheader && tmpl.Subheader != "" {
		buf.Write(FONT_B)
		buf.WriteString(tmpl.Subheader + "\n")
		buf.Write(FONT_A)
	}

	buf.WriteString(separator)

	// ── Queue number ─────────────────────────────────────────────────────────
	buf.Write(FONT_B)
	title := tmpl.Title
	if title == "" {
		title = "NOMOR ANTRIAN ANDA"
	}
	buf.WriteString(title + "\n")
	buf.Write(FONT_A)

	buf.Write(FEED_LINE)
	buf.Write(numberSize)
	buf.WriteString(data.QueueNumber + "\n")
	buf.Write(SIZE_NORMAL)

	// Type name (optional)
	if tmpl.ShowType {
		buf.Write(FEED_LINE)
		buf.Write(BOLD_ON)
		buf.WriteString(data.TypeName + "\n")
		buf.Write(BOLD_OFF)
	}

	buf.WriteString(separator)

	// ── DateTime (optional) ──────────────────────────────────────────────────
	if tmpl.ShowDatetime {
		buf.Write(FONT_B)
		buf.WriteString(data.DateTime + "\n")
		buf.Write(FONT_A)
	}

	// ── Footer (optional) ────────────────────────────────────────────────────
	if tmpl.ShowFooter {
		buf.WriteString(separator)
		buf.Write(FONT_B)
		footer1 := tmpl.Footer1
		if footer1 == "" {
			footer1 = "Mohon menunggu hingga"
		}
		buf.WriteString(footer1 + "\n")
		footer2 := tmpl.Footer2
		if footer2 == "" {
			footer2 = "nomor Anda dipanggil"
		}
		buf.WriteString(footer2 + "\n")
		buf.Write(FONT_A)
	}

	// Thanks (optional)
	if tmpl.ShowThanks {
		buf.WriteString("\n")
		buf.Write(FONT_B)
		thanks := tmpl.Thanks
		if thanks == "" {
			thanks = "Terima kasih"
		}
		buf.WriteString(thanks + "\n")
		buf.Write(FONT_A)
	}

	// ── Feed + partial cut (single command, no wasted lines) ─────────────────
	// GS V 0x42 n  →  feed n lines then partial cut
	buf.Write([]byte{GS, 'V', 0x42, byte(feedLines)})

	return p.sendToPrinter(buf.Bytes())
}

// PrintTicketSimple prints a ticket with default template (for backward compatibility)
func (p *Printer) PrintTicketSimple(data TicketData) error {
	return p.PrintTicket(data, DefaultTemplate())
}

// sendToPrinter sends raw data to the Windows printer using PowerShell
func (p *Printer) sendToPrinter(data []byte) error {
	printerName := p.config.PrinterName
	if printerName == "" {
		printerName = "ECO80"
	}

	// Create temp file with raw print data
	tempDir := os.TempDir()
	tempFile := filepath.Join(tempDir, fmt.Sprintf("ticket_%d.bin", time.Now().UnixNano()))

	if err := os.WriteFile(tempFile, data, 0644); err != nil {
		return fmt.Errorf("failed to write temp file: %w", err)
	}
	defer os.Remove(tempFile)

	// Use PowerShell to send raw data to printer
	// This is the most reliable method for Windows
	psScript := fmt.Sprintf(`
$printerName = '%s'
$filePath = '%s'

# Get printer
$printer = Get-WmiObject -Query "SELECT * FROM Win32_Printer WHERE Name='$printerName'" -ErrorAction SilentlyContinue

if ($printer -eq $null) {
    # Try without exact match
    $printer = Get-WmiObject -Query "SELECT * FROM Win32_Printer WHERE Name LIKE '%%$printerName%%'" -ErrorAction SilentlyContinue
}

if ($printer -eq $null) {
    Write-Error "Printer '$printerName' not found"
    exit 1
}

# Get printer port
$portName = $printer.PortName

# Read file content as bytes
$bytes = [System.IO.File]::ReadAllBytes($filePath)

# Try direct port write first (works for USB printers)
try {
    $port = [System.IO.Ports.SerialPort]::GetPortNames() | Where-Object { $_ -eq $portName }
    if ($port) {
        $serialPort = New-Object System.IO.Ports.SerialPort $portName, 9600
        $serialPort.Open()
        $serialPort.Write($bytes, 0, $bytes.Length)
        $serialPort.Close()
        exit 0
    }
} catch {}

# Fallback: Use raw print job via .NET
Add-Type -AssemblyName System.Drawing

$doc = New-Object System.Drawing.Printing.PrintDocument
$doc.PrinterSettings.PrinterName = $printerName

# For raw printing, we use RawPrinterHelper
$helper = @"
using System;
using System.Runtime.InteropServices;

public class RawPrinterHelper
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public class DOCINFOA
    {
        [MarshalAs(UnmanagedType.LPStr)] public string pDocName;
        [MarshalAs(UnmanagedType.LPStr)] public string pOutputFile;
        [MarshalAs(UnmanagedType.LPStr)] public string pDataType;
    }

    [DllImport("winspool.Drv", EntryPoint = "OpenPrinterA", CharSet = CharSet.Ansi, SetLastError = true)]
    public static extern bool OpenPrinter([MarshalAs(UnmanagedType.LPStr)] string szPrinter, out IntPtr hPrinter, IntPtr pd);

    [DllImport("winspool.Drv", EntryPoint = "ClosePrinter", SetLastError = true)]
    public static extern bool ClosePrinter(IntPtr hPrinter);

    [DllImport("winspool.Drv", EntryPoint = "StartDocPrinterA", CharSet = CharSet.Ansi, SetLastError = true)]
    public static extern bool StartDocPrinter(IntPtr hPrinter, Int32 level, [In, MarshalAs(UnmanagedType.LPStruct)] DOCINFOA di);

    [DllImport("winspool.Drv", EntryPoint = "EndDocPrinter", SetLastError = true)]
    public static extern bool EndDocPrinter(IntPtr hPrinter);

    [DllImport("winspool.Drv", EntryPoint = "StartPagePrinter", SetLastError = true)]
    public static extern bool StartPagePrinter(IntPtr hPrinter);

    [DllImport("winspool.Drv", EntryPoint = "EndPagePrinter", SetLastError = true)]
    public static extern bool EndPagePrinter(IntPtr hPrinter);

    [DllImport("winspool.Drv", EntryPoint = "WritePrinter", SetLastError = true)]
    public static extern bool WritePrinter(IntPtr hPrinter, IntPtr pBytes, Int32 dwCount, out Int32 dwWritten);

    public static bool SendBytesToPrinter(string szPrinterName, byte[] bytes)
    {
        IntPtr hPrinter = IntPtr.Zero;
        DOCINFOA di = new DOCINFOA();
        di.pDocName = "Queue Ticket";
        di.pDataType = "RAW";

        if (OpenPrinter(szPrinterName.Normalize(), out hPrinter, IntPtr.Zero))
        {
            if (StartDocPrinter(hPrinter, 1, di))
            {
                if (StartPagePrinter(hPrinter))
                {
                    IntPtr pUnmanagedBytes = Marshal.AllocCoTaskMem(bytes.Length);
                    Marshal.Copy(bytes, 0, pUnmanagedBytes, bytes.Length);
                    int dwWritten;
                    WritePrinter(hPrinter, pUnmanagedBytes, bytes.Length, out dwWritten);
                    Marshal.FreeCoTaskMem(pUnmanagedBytes);
                    EndPagePrinter(hPrinter);
                }
                EndDocPrinter(hPrinter);
            }
            ClosePrinter(hPrinter);
            return true;
        }
        return false;
    }
}
"@

Add-Type -TypeDefinition $helper -Language CSharp -ErrorAction SilentlyContinue

[RawPrinterHelper]::SendBytesToPrinter($printerName, $bytes)
`, printerName, escapeForPS(tempFile))

	cmd := exec.Command("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", psScript)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("print failed: %v, output: %s", err, string(output))
	}

	return nil
}

// escapeForPS escapes a string for use in PowerShell
func escapeForPS(s string) string {
	result := ""
	for _, c := range s {
		if c == '\\' {
			result += "\\\\"
		} else if c == '\'' {
			result += "''"
		} else {
			result += string(c)
		}
	}
	return result
}

// TestPrint sends a test print to verify printer connection
func (p *Printer) TestPrint() error {
	var buf bytes.Buffer

	buf.Write(INIT)
	buf.Write(ALIGN_CENTER)
	buf.Write(BOLD_ON)
	buf.WriteString("=== TEST PRINT ===\n")
	buf.Write(BOLD_OFF)
	buf.WriteString("\n")
	buf.WriteString("Printer: " + p.config.PrinterName + "\n")
	buf.WriteString("Time: " + time.Now().Format("02/01/2006 15:04:05") + "\n")
	buf.WriteString("\n")
	buf.WriteString("Jika Anda melihat ini,\n")
	buf.WriteString("printer berfungsi dengan baik!\n")
	buf.Write([]byte{GS, 'V', 0x42, 2}) // feed 2 lines + cut
	return p.sendToPrinter(buf.Bytes())
}

// IsEnabled returns whether printing is enabled
func (p *Printer) IsEnabled() bool {
	return p.config.Enabled
}

// GetPrinterName returns the configured printer name
func (p *Printer) GetPrinterName() string {
	return p.config.PrinterName
}
