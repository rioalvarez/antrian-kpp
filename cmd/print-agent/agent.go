package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"strings"
	"time"

	"queue-system/internal/printer"
)

type PrintAgent struct {
	config  *AgentConfig
	printer *printer.Printer
	client  *http.Client
}

type PrintJobResponse struct {
	ID           int64  `json:"id"`
	QueueNumber  string `json:"queue_number"`
	TypeName     string `json:"type_name"`
	DateTime     string `json:"date_time"`
	TemplateJSON string `json:"template_json"`
	Status       string `json:"status"`
}

type SSEEvent struct {
	Type string          `json:"type"`
	Data json.RawMessage `json:"data"`
}

type PrintJobEvent struct {
	JobID       int64  `json:"job_id"`
	QueueNumber string `json:"queue_number"`
}

func NewPrintAgent(cfg *AgentConfig) *PrintAgent {
	return &PrintAgent{
		config: cfg,
		printer: printer.New(printer.PrinterConfig{
			Enabled:     true,
			PrinterName: cfg.PrinterName,
		}),
		client: &http.Client{Timeout: 30 * time.Second},
	}
}

// Run starts the agent loop: catch up pending jobs, then subscribe to SSE.
// On disconnection, it waits and retries.
func (a *PrintAgent) Run(stop <-chan struct{}) {
	for {
		log.Println("Catching up pending jobs...")
		a.catchUpPendingJobs()

		log.Printf("Connecting to SSE at %s...", a.config.ServerURL)
		err := a.subscribeSSE(stop)
		if err != nil {
			log.Printf("SSE connection lost: %v", err)
		}

		// Check if we should stop
		select {
		case <-stop:
			log.Println("Agent stopping...")
			return
		default:
		}

		delay := time.Duration(a.config.RetryDelay) * time.Second
		log.Printf("Reconnecting in %v...", delay)
		select {
		case <-time.After(delay):
		case <-stop:
			return
		}
	}
}

func (a *PrintAgent) catchUpPendingJobs() {
	url := fmt.Sprintf("%s/api/print-agent/jobs/pending", a.config.ServerURL)
	resp, err := a.client.Get(url)
	if err != nil {
		log.Printf("Failed to fetch pending jobs: %v", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		log.Printf("Pending jobs returned status %d", resp.StatusCode)
		return
	}

	var jobs []PrintJobResponse
	if err := json.NewDecoder(resp.Body).Decode(&jobs); err != nil {
		log.Printf("Failed to decode pending jobs: %v", err)
		return
	}

	log.Printf("Found %d pending jobs", len(jobs))
	for _, job := range jobs {
		a.processJob(job.ID)
	}
}

func (a *PrintAgent) subscribeSSE(stop <-chan struct{}) error {
	url := fmt.Sprintf("%s/api/print-agent/sse?agent_id=%s", a.config.ServerURL, a.config.AgentID)

	// Use a client without timeout for SSE (long-lived connection)
	sseClient := &http.Client{}
	resp, err := sseClient.Get(url)
	if err != nil {
		return fmt.Errorf("failed to connect SSE: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("SSE returned status %d", resp.StatusCode)
	}

	log.Println("SSE connected, waiting for print jobs...")

	scanner := bufio.NewScanner(resp.Body)
	// Increase buffer for large messages
	scanner.Buffer(make([]byte, 0, 64*1024), 64*1024)

	for {
		// Check stop signal
		select {
		case <-stop:
			return nil
		default:
		}

		if !scanner.Scan() {
			if err := scanner.Err(); err != nil {
				return fmt.Errorf("SSE read error: %w", err)
			}
			return fmt.Errorf("SSE stream ended")
		}

		line := scanner.Text()

		// Skip heartbeat comments and empty lines
		if line == "" || strings.HasPrefix(line, ":") {
			continue
		}

		// Parse SSE event
		if strings.HasPrefix(line, "event: ") {
			// Read the next "data:" line
			if !scanner.Scan() {
				break
			}
			dataLine := scanner.Text()
			if !strings.HasPrefix(dataLine, "data: ") {
				continue
			}

			data := strings.TrimPrefix(dataLine, "data: ")
			a.handleSSEMessage(data)
		}
	}

	return fmt.Errorf("SSE stream ended")
}

func (a *PrintAgent) handleSSEMessage(data string) {
	var event SSEEvent
	if err := json.Unmarshal([]byte(data), &event); err != nil {
		log.Printf("Failed to parse SSE message: %v", err)
		return
	}

	switch event.Type {
	case "print_job":
		var pjEvent PrintJobEvent
		if err := json.Unmarshal(event.Data, &pjEvent); err != nil {
			log.Printf("Failed to parse print_job event: %v", err)
			return
		}
		log.Printf("Received print job #%d for %s", pjEvent.JobID, pjEvent.QueueNumber)
		go a.processJob(pjEvent.JobID)
	}
}

func (a *PrintAgent) processJob(jobID int64) {
	// 1. Claim the job
	claimed, err := a.claimJob(jobID)
	if err != nil {
		log.Printf("Failed to claim job #%d: %v (likely already claimed)", jobID, err)
		return
	}

	log.Printf("Claimed job #%d: %s", jobID, claimed.QueueNumber)

	// 2. Parse template from JSON
	var tmpl printer.TicketTemplate
	if err := json.Unmarshal([]byte(claimed.TemplateJSON), &tmpl); err != nil {
		log.Printf("Failed to parse template for job #%d: %v", jobID, err)
		a.failJob(jobID, "failed to parse template: "+err.Error())
		return
	}

	// 3. Print the ticket
	err = a.printer.PrintTicket(printer.TicketData{
		QueueNumber: claimed.QueueNumber,
		TypeName:    claimed.TypeName,
		DateTime:    claimed.DateTime,
	}, tmpl)

	if err != nil {
		log.Printf("Print failed for job #%d: %v", jobID, err)
		a.failJob(jobID, err.Error())
		return
	}

	// 4. Mark as completed
	a.completeJob(jobID)
	log.Printf("Job #%d printed successfully: %s", jobID, claimed.QueueNumber)
}

func (a *PrintAgent) claimJob(jobID int64) (*PrintJobResponse, error) {
	url := fmt.Sprintf("%s/api/print-agent/job/%d/claim", a.config.ServerURL, jobID)
	body, _ := json.Marshal(map[string]string{"agent_id": a.config.AgentID})

	resp, err := a.client.Post(url, "application/json", bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("claim returned %d: %s", resp.StatusCode, string(respBody))
	}

	var job PrintJobResponse
	if err := json.NewDecoder(resp.Body).Decode(&job); err != nil {
		return nil, err
	}
	return &job, nil
}

func (a *PrintAgent) completeJob(jobID int64) {
	url := fmt.Sprintf("%s/api/print-agent/job/%d/complete", a.config.ServerURL, jobID)
	resp, err := a.client.Post(url, "application/json", strings.NewReader("{}"))
	if err != nil {
		log.Printf("Failed to mark job #%d complete: %v", jobID, err)
		return
	}
	resp.Body.Close()
}

func (a *PrintAgent) failJob(jobID int64, errMsg string) {
	url := fmt.Sprintf("%s/api/print-agent/job/%d/fail", a.config.ServerURL, jobID)
	body, _ := json.Marshal(map[string]string{"error": errMsg})
	resp, err := a.client.Post(url, "application/json", bytes.NewReader(body))
	if err != nil {
		log.Printf("Failed to mark job #%d failed: %v", jobID, err)
		return
	}
	resp.Body.Close()
}
