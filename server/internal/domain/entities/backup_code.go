package entities

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"math/rand"
	"time"

	"github.com/google/uuid"
)

// BackupCode represents a one-time-use backup/recovery code for MFA fallback.
type BackupCode struct {
	ID        uuid.UUID
	UserID    uuid.UUID
	CodeHash  string // SHA-256 hash of the code (never store plaintext)
	IsUsed    bool
	CreatedAt time.Time
	UsedAt    *time.Time
}

// GenerateBackupCodes creates a set of one-time backup codes.
// Returns the BackupCode entities (with hashes) and the plaintext codes (to show once).
func GenerateBackupCodes(userID uuid.UUID, count int) ([]*BackupCode, []string) {
	codes := make([]*BackupCode, 0, count)
	plaintextCodes := make([]string, 0, count)

	for i := 0; i < count; i++ {
		plaintext := generateBackupCode()
		plaintextCodes = append(plaintextCodes, plaintext)

		id, err := uuid.NewV7()
		if err != nil {
			panic("failed to generate UUID for BackupCode")
		}

		codes = append(codes, &BackupCode{
			ID:        id,
			UserID:    userID,
			CodeHash:  HashBackupCode(plaintext),
			IsUsed:    false,
			CreatedAt: time.Now().UTC(),
			UsedAt:    nil,
		})
	}

	return codes, plaintextCodes
}

// HashBackupCode creates a SHA-256 hash of a backup code.
func HashBackupCode(code string) string {
	hash := sha256.Sum256([]byte(code))
	return hex.EncodeToString(hash[:])
}

// generateBackupCode generates a random 8-character alphanumeric code in format XXXX-XXXX.
func generateBackupCode() string {
	const charset = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789" // excludes easily confused chars (0,O,1,I)
	b := make([]byte, 8)
	for i := range b {
		b[i] = charset[rand.Intn(len(charset))]
	}
	return fmt.Sprintf("%s-%s", string(b[:4]), string(b[4:]))
}
