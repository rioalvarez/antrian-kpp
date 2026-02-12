package main

import (
	"flag"
	"log"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	configPath := flag.String("config", "config.yaml", "Path to agent config file")
	flag.Parse()

	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.Println("Starting Print Agent...")

	cfg, err := LoadAgentConfig(*configPath)
	if err != nil {
		log.Fatalf("Failed to load config: %v", err)
	}

	log.Printf("Agent ID:     %s", cfg.AgentID)
	log.Printf("Server URL:   %s", cfg.ServerURL)
	log.Printf("Printer Name: %s", cfg.PrinterName)
	log.Printf("Retry Delay:  %ds", cfg.RetryDelay)

	agent := NewPrintAgent(cfg)

	stop := make(chan struct{})

	// Handle OS signals for graceful shutdown
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-quit
		log.Println("Shutdown signal received...")
		close(stop)
	}()

	agent.Run(stop)
	log.Println("Print Agent stopped.")
}
