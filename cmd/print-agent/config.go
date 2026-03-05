package main

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

type AgentConfig struct {
	AgentID     string `yaml:"agent_id"`
	ServerURL   string `yaml:"server_url"`
	PrinterName string `yaml:"printer_name"`
	RetryDelay  int    `yaml:"retry_delay"`
	PaperSize   string `yaml:"paper_size"`  // "80mm" (default) or "58mm"
	FeedLines   int    `yaml:"feed_lines"`  // lines to feed before cut (default 1)
}

func DefaultAgentConfig() *AgentConfig {
	return &AgentConfig{
		AgentID:     "printer-1",
		ServerURL:   "http://localhost:8080",
		PrinterName: "ECO80",
		RetryDelay:  5,
		PaperSize:   "80mm",
		FeedLines:   1,
	}
}

func LoadAgentConfig(path string) (*AgentConfig, error) {
	cfg := DefaultAgentConfig()

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("config file not found: %s", path)
		}
		return nil, err
	}

	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	if cfg.ServerURL == "" {
		return nil, fmt.Errorf("server_url is required in config")
	}

	return cfg, nil
}
