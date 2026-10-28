// Command powdemo serves the benchmark and demo pages with a real issuer and
// verifier behind them. It is a development tool, not a production server.
package main

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"sync"
	"time"

	pow "github.com/next-moe/nextmoe-pow/go"
)

type memoryStore struct {
	mu   sync.Mutex
	seen map[string]time.Time
}

func (s *memoryStore) Claim(_ context.Context, key string, ttl time.Duration) (bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now()
	if until, ok := s.seen[key]; ok && now.Before(until) {
		return false, nil
	}
	s.seen[key] = now.Add(ttl)
	return true, nil
}

func uintParam(r *http.Request, name string, def uint64, bits int) (uint64, error) {
	v := r.URL.Query().Get(name)
	if v == "" {
		return def, nil
	}
	return strconv.ParseUint(v, 10, bits)
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

func main() {
	addr := flag.String("addr", ":8787", "listen address")
	root := flag.String("root", ".", "repository root to serve")
	results := flag.String("results", "bench/results", "directory for posted benchmark results")
	flag.Parse()

	key := make([]byte, 32)
	rand.Read(key)
	keys := [][]byte{key}
	store := &memoryStore{seen: map[string]time.Time{}}
	label := regexp.MustCompile(`[^a-zA-Z0-9_-]+`)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/challenge", func(w http.ResponseWriter, r *http.Request) {
		alg, errA := uintParam(r, "alg", uint64(pow.SHA256), 8)
		m, errM := uintParam(r, "m", 0, 32)
		t, errT := uintParam(r, "t", 0, 8)
		k, errK := uintParam(r, "k", 1, 8)
		d, errD := uintParam(r, "d", 1, 32)
		if err := firstErr(errA, errM, errT, errK, errD); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		p := pow.Params{Alg: pow.Alg(alg), MemoryKiB: uint32(m), Iterations: uint8(t), K: uint8(k), Difficulty: uint32(d), TTL: 10 * time.Minute}
		scope := pow.Scope{Site: r.URL.Query().Get("site"), Action: r.URL.Query().Get("action")}
		token, err := pow.Issue(key, p, scope, time.Now())
		if err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"token": token})
	})
	mux.HandleFunc("POST /api/verify", func(w http.ResponseWriter, r *http.Request) {
		var body struct{ Solution, Site, Action string }
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		started := time.Now()
		_, err := pow.VerifyOnce(r.Context(), store, keys, body.Solution, pow.Scope{Site: body.Site, Action: body.Action}, started)
		elapsed := time.Since(started)
		code := "ok"
		if err != nil {
			code = err.Error()
		}
		writeJSON(w, http.StatusOK, map[string]any{"code": code, "verify_us": elapsed.Microseconds()})
	})
	mux.HandleFunc("POST /api/results", func(w http.ResponseWriter, r *http.Request) {
		var body map[string]any
		if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4<<20)).Decode(&body); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		name := fmt.Sprintf("%s-%s.json", time.Now().Format("20060102-150405"), label.ReplaceAllString(fmt.Sprint(body["label"]), "_"))
		raw, _ := json.MarshalIndent(body, "", "  ")
		if err := os.WriteFile(filepath.Join(*results, name), append(raw, '\n'), 0o644); err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
			return
		}
		log.Printf("saved %s", name)
		writeJSON(w, http.StatusOK, map[string]string{"file": name})
	})
	mux.Handle("/", http.FileServer(http.Dir(*root)))

	log.Printf("serving %s on %s (bench: /bench/)", *root, *addr)
	log.Fatal(http.ListenAndServe(*addr, mux))
}

func firstErr(errs ...error) error {
	for _, err := range errs {
		if err != nil {
			return err
		}
	}
	return nil
}
