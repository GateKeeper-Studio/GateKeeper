package session

import (
	"context"
	"os"
	"testing"

	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_Session_InvalidToken(t *testing.T) {
	h := &Handler{}
	_, err := h.Handler(context.Background(), Command{AccessToken: "invalid-token"})

	require.Error(t, err)
}

func TestHandler_Session_Success(t *testing.T) {
	os.Setenv("JWT_SECRET", "test-secret-for-jwt-token-signing")
	defer os.Unsetenv("JWT_SECRET")

	userID, _ := uuid.NewV7()
	appID, _ := uuid.NewV7()

	token, err := application_utils.CreateToken(application_utils.JWTClaims{
		UserID:        userID,
		FirstName:     "John",
		LastName:      "Doe",
		DisplayName:   "johndoe",
		Email:         "john@test.com",
		ApplicationID: appID,
	})
	require.NoError(t, err)

	h := &Handler{}
	resp, err := h.Handler(context.Background(), Command{AccessToken: token})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, userID, resp.User.ID)
	assert.Equal(t, "John", resp.User.FirstName)
	assert.Equal(t, "Doe", resp.User.LastName)
	assert.Equal(t, "johndoe", resp.User.DisplayName)
	assert.Equal(t, "john@test.com", resp.User.Email)
	assert.Equal(t, appID, resp.User.ApplicationID)
	assert.Equal(t, token, resp.AccessToken)
}

func TestHandler_Session_WrongSecret(t *testing.T) {
	os.Setenv("JWT_SECRET", "correct-secret-key-for-signing!!")
	userID, _ := uuid.NewV7()
	appID, _ := uuid.NewV7()

	token, err := application_utils.CreateToken(application_utils.JWTClaims{
		UserID:        userID,
		FirstName:     "Jane",
		LastName:      "Doe",
		DisplayName:   "janedoe",
		Email:         "jane@test.com",
		ApplicationID: appID,
	})
	require.NoError(t, err)

	// Decode with a different secret
	os.Setenv("JWT_SECRET", "wrong-secret-key-for-decoding!!!")

	h := &Handler{}
	_, err = h.Handler(context.Background(), Command{AccessToken: token})

	require.Error(t, err)
	os.Unsetenv("JWT_SECRET")
}
