package main

// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

import (
	"crypto/md5"
	"crypto/sha1"
	"crypto/tls"
	"database/sql"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
)

// GO-001: SQL Injection via string concatenation
func getUserByName(db *sql.DB, name string) (*User, error) {
	query := "SELECT * FROM users WHERE name = '" + name + "'" // BAD: String concat
	rows, _ := db.Query(query)

	// Also bad: fmt.Sprintf
	q := fmt.Sprintf("SELECT * FROM users WHERE id = %s", name) // BAD: Sprintf
	db.Exec(q)

	return nil, nil
}

// GO-002: Insecure TLS Configuration
func createInsecureClient() *http.Client {
	tlsConfig := &tls.Config{
		InsecureSkipVerify: true,  // BAD: Skip cert verification
		MinVersion:         tls.VersionTLS10, // BAD: Weak TLS version
	}
	return &http.Client{Transport: &http.Transport{TLSClientConfig: tlsConfig}}
}

// GO-003: Command Injection
func runCommand(userInput string) {
	cmd := exec.Command("sh", "-c", "echo " + userInput) // BAD: String concat
	cmd.Run()
}

// GO-004: Path Traversal
func serveFile(w http.ResponseWriter, r *http.Request) {
	filename := filepath.Join("/var/www", r.URL.Path) // BAD: User input in path
	data, _ := os.Open(r.FormValue("file"))           // BAD: User input in Open
	http.ServeFile(w, r, r.URL.Path)                  // BAD: Direct URL use
}

// GO-005: Weak Cryptography
func hashPassword(password string) []byte {
	h := md5.New()  // BAD: MD5 is weak
	h.Write([]byte(password))
	return h.Sum(nil)
}

func hashData(data string) []byte {
	h := sha1.New() // BAD: SHA1 is weak
	h.Write([]byte(data))
	return h.Sum(nil)
}

// SAFE PATTERNS (should not trigger)
func safeExample(db *sql.DB) {
	// Safe: Parameterized query
	db.Query("SELECT * FROM users WHERE id = ?", id)

	// Safe: TLS 1.2+
	tlsConfig := &tls.Config{
		MinVersion: tls.VersionTLS12,
	}

	// Safe: filepath.Clean
	cleanPath := filepath.Clean(userPath)
	if strings.HasPrefix(cleanPath, "/allowed/") {
		// proceed
	}
}
