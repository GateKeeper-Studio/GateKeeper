package accountchangepassword

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockChangePassRepo struct{ mock.Mock }

func (m *mockChangePassRepo) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.TenantUser), args.Error(1)
}

func (m *mockChangePassRepo) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserCredentials), args.Error(1)
}

func (m *mockChangePassRepo) UpdateUserCredentials(ctx context.Context, creds *entities.UserCredentials) error {
	return m.Called(ctx, creds).Error(0)
}

func (m *mockChangePassRepo) GetApplicationByID(ctx context.Context, appID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, appID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

func (m *mockChangePassRepo) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockChangePassRepo) RevokeAllUserSessions(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockChangePassRepo) AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error {
	return m.Called(ctx, auditLog).Error(0)
}

func (m *mockChangePassRepo) GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error) {
	args := m.Called(ctx, id)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Tenant), args.Error(1)
}

var _ IRepository = (*mockChangePassRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ChangePassword_UserNotFound(t *testing.T) {
	repo := new(mockChangePassRepo)
	userID := uuid.New()

	repo.On("GetUserByID", mock.Anything, userID).Return(nil, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:          userID,
		ApplicationID:   uuid.New(),
		CurrentPassword: "OldPass1!",
		NewPassword:     "NewPass1!",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, &errors.ErrUserNotFound, err)
	repo.AssertExpectations(t)
}

func TestHandler_ChangePassword_CredentialsNotFound(t *testing.T) {
	repo := new(mockChangePassRepo)
	userID := uuid.New()
	user := &entities.TenantUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, userID).Return(nil, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:          userID,
		ApplicationID:   uuid.New(),
		CurrentPassword: "OldPass1!",
		NewPassword:     "NewPass1!",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, &errors.ErrUserCredentialsNotFound, err)
	repo.AssertExpectations(t)
}

func TestHandler_ChangePassword_IncorrectCurrentPassword(t *testing.T) {
	repo := new(mockChangePassRepo)
	userID := uuid.New()
	appID := uuid.New()
	user := &entities.TenantUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}
	creds := &entities.UserCredentials{
		ID:                uuid.New(),
		UserID:            userID,
		PasswordHash:      "invalid-hash-that-wont-match",
		PasswordAlgorithm: constants.UserCredentialsPasswordAlgorithmArgon2i,
		CreatedAt:         time.Now().UTC(),
	}

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, userID).Return(creds, nil)
	// ComparePassword will error because hash is not a valid argon2 hash,
	// so the handler returns the error before reaching AddAuditLog.

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:          userID,
		ApplicationID:   appID,
		CurrentPassword: "WrongPassword1!",
		NewPassword:     "NewPass1!",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	repo.AssertExpectations(t)
}

func TestHandler_ChangePassword_GetUserDBError(t *testing.T) {
	repo := new(mockChangePassRepo)
	userID := uuid.New()
	dbErr := fmt.Errorf("db connection timeout")

	repo.On("GetUserByID", mock.Anything, userID).Return(nil, dbErr)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:          userID,
		ApplicationID:   uuid.New(),
		CurrentPassword: "OldPass1!",
		NewPassword:     "NewPass1!",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "db connection timeout", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ChangePassword_GetCredentialsDBError(t *testing.T) {
	repo := new(mockChangePassRepo)
	userID := uuid.New()
	user := &entities.TenantUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}
	dbErr := fmt.Errorf("query failed")

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, userID).Return(nil, dbErr)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:          userID,
		ApplicationID:   uuid.New(),
		CurrentPassword: "OldPass1!",
		NewPassword:     "NewPass1!",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "query failed", err.Error())
	repo.AssertExpectations(t)
}

func TestValidatePasswordStrength(t *testing.T) {
	tests := []struct {
		name     string
		password string
		expected bool
	}{
		{"valid password", "MyP@ssw0rd", true},
		{"another valid", "Str0ng!Pass", true},
		{"too short", "Ab1!", false},
		{"no uppercase", "myp@ssw0rd", false},
		{"no lowercase", "MYP@SSW0RD", false},
		{"no digit", "MyP@ssword", false},
		{"no special char", "MyPassw0rd", false},
		{"empty string", "", false},
		{"just spaces", "        ", false},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result := validatePasswordStrength(tc.password)
			assert.Equal(t, tc.expected, result)
		})
	}
}
