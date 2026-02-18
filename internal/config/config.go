package config

import (
	"os"
	"strings"
	"time"

	"golang.org/x/crypto/bcrypt"
	"gopkg.in/yaml.v3"
)

type Config struct {
	Server   ServerConfig   `yaml:"server"`
	Database DatabaseConfig `yaml:"database"`
	Logging  LoggingConfig  `yaml:"logging"`
	Queue    QueueConfig    `yaml:"queue"`
	Audio    AudioConfig    `yaml:"audio"`
	Security SecurityConfig `yaml:"security"`
	Printer  PrinterConfig  `yaml:"printer"`
}

type PrinterConfig struct {
	Enabled       bool   `yaml:"enabled"`
	PrinterName   string `yaml:"printer_name"`
	RemoteEnabled bool   `yaml:"remote_enabled"`
}

type ServerConfig struct {
	Port         int           `yaml:"port"`
	Host         string        `yaml:"host"`
	ReadTimeout  time.Duration `yaml:"read_timeout"`
	WriteTimeout time.Duration `yaml:"write_timeout"`
}

type DatabaseConfig struct {
	Path         string `yaml:"path"`
	MaxOpenConns int    `yaml:"max_open_conns"`
	MaxIdleConns int    `yaml:"max_idle_conns"`
}

type LoggingConfig struct {
	Level    string `yaml:"level"`
	Output   string `yaml:"output"`
	FilePath string `yaml:"file_path"`
}

type QueueConfig struct {
	Prefix          string `yaml:"prefix"`
	StartNumber     int    `yaml:"start_number"`
	ResetDaily      bool   `yaml:"reset_daily"`
	AutoCancelHours int    `yaml:"auto_cancel_hours"`
}

type AudioConfig struct {
	Enabled  bool   `yaml:"enabled"`
	BellFile string `yaml:"bell_file"`
}

type SecurityConfig struct {
	AdminPassword  string `yaml:"admin_password"`
	SessionTimeout int    `yaml:"session_timeout"`
}

func DefaultConfig() *Config {
	return &Config{
		Server: ServerConfig{
			Port:         8080,
			Host:         "0.0.0.0",
			ReadTimeout:  30 * time.Second,
			WriteTimeout: 30 * time.Second,
		},
		Database: DatabaseConfig{
			Path:         "./data/queue.db",
			MaxOpenConns: 25,
			MaxIdleConns: 5,
		},
		Logging: LoggingConfig{
			Level:    "info",
			Output:   "stdout",
			FilePath: "./data/logs/app.log",
		},
		Queue: QueueConfig{
			Prefix:          "A",
			StartNumber:     1,
			ResetDaily:      true,
			AutoCancelHours: 24,
		},
		Audio: AudioConfig{
			Enabled:  true,
			BellFile: "/static/audio/bell.mp3",
		},
		Security: SecurityConfig{
			AdminPassword:  "admin123",
			SessionTimeout: 3600,
		},
		Printer: PrinterConfig{
			Enabled:     true,
			PrinterName: "ECO80",
		},
	}
}

func Load(path string) (*Config, error) {
	cfg := DefaultConfig()

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil
		}
		return nil, err
	}

	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, err
	}

	return cfg, nil
}

func (c *Config) Save(path string) error {
	data, err := yaml.Marshal(c)
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

// isBcryptHash returns true if the string looks like a bcrypt hash.
func isBcryptHash(s string) bool {
	return strings.HasPrefix(s, "$2a$") ||
		strings.HasPrefix(s, "$2b$") ||
		strings.HasPrefix(s, "$2y$")
}

// HashPasswordIfPlain checks if admin_password is plaintext.
// If so, it hashes it with bcrypt, updates the config, and saves the file.
// Call this once after loading config.
func (c *Config) HashPasswordIfPlain(configPath string) error {
	if isBcryptHash(c.Security.AdminPassword) {
		return nil // already hashed, nothing to do
	}
	hash, err := bcrypt.GenerateFromPassword([]byte(c.Security.AdminPassword), bcrypt.DefaultCost)
	if err != nil {
		return err
	}
	c.Security.AdminPassword = string(hash)
	return c.Save(configPath)
}

// VerifyAdminPassword checks a plaintext password against the stored bcrypt hash.
func (c *Config) VerifyAdminPassword(password string) bool {
	err := bcrypt.CompareHashAndPassword([]byte(c.Security.AdminPassword), []byte(password))
	return err == nil
}
