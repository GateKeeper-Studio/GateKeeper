package entities

import (
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/google/uuid"
)

// UserCredentials represents the credentials associated with an application user.
type UserCredentials struct {
	ID                uuid.UUID
	UserID            uuid.UUID // references ApplicationUser.ID
	PasswordHash      string    // hashed password
	PasswordAlgorithm string    // e.g., "bcrypt", "argon2"
	ShouldChangePass  bool      // indicates if the user should change their password on next login
	CreatedAt         time.Time
	UpdatedAt         *time.Time
}

func NewUserCredentials(userID uuid.UUID, passwordHash string, shouldChangePass bool) *UserCredentials {
	id, err := uuid.NewV7()

	if err != nil {
		panic("failed to generate UUID for UserCredentials")
	}

	return &UserCredentials{
		ID:                id,
		UserID:            userID,
		PasswordHash:      passwordHash,
		PasswordAlgorithm: constants.UserCredentialsPasswordAlgorithmArgon2i, // default algorithm
		ShouldChangePass:  shouldChangePass,
		CreatedAt:         time.Now(),
		UpdatedAt:         nil,
	}
}
