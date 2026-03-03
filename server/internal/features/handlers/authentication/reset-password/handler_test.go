package resetpassword

import (
	"context"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockResetPasswordRepo struct{ mock.Mock }

func (m *mockResetPasswordRepo) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockResetPasswordRepo) GetPasswordResetByTokenID(ctx context.Context, tokenID uuid.UUID) (*entities.PasswordResetToken, error) {
	args := m.Called(ctx, tokenID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.PasswordResetToken), args.Error(1)
}

func (m *mockResetPasswordRepo) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, user)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockResetPasswordRepo) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

func (m *mockResetPasswordRepo) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockResetPasswordRepo) DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockResetPasswordRepo) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserCredentials), args.Error(1)
}

func (m *mockResetPasswordRepo) UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	return m.Called(ctx, userCredentials).Error(0)
}

// Compile-time check
var _ IRepository = (*mockResetPasswordRepo)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newValidResetToken(userID uuid.UUID) *entities.PasswordResetToken {
	id, _ := uuid.NewV7()
	return &entities.PasswordResetToken{
		ID:        id,
		UserID:    userID,
		Token:     "valid-reset-token-string",
		CreatedAt: time.Now().UTC(),
		ExpiresAt: time.Now().UTC().Add(15 * time.Minute),
	}
}

func newResetUser(appID uuid.UUID) *entities.ApplicationUser {
	id, _ := uuid.NewV7()
	return &entities.ApplicationUser{
		ID:               id,
		ApplicationID:    appID,
		Email:            "user@example.com",
		IsActive:         true,
		IsEmailConfirmed: true,
		CreatedAt:        time.Now().UTC(),
	}
}

func newResetApp(orgID uuid.UUID) *entities.Application {
	return &entities.Application{
		ID:                 uuid.New(),
		OrganizationID:     orgID,
		Name:               "Test App",
		IsActive:           true,
		PasswordHashSecret: "test-salt-key-for-password-hashing",
	}
}

func newResetCredentials(userID uuid.UUID) *entities.UserCredentials {
	id, _ := uuid.NewV7()
	return &entities.UserCredentials{
		ID:               id,
		UserID:           userID,
		PasswordHash:     "old-password-hash",
		ShouldChangePass: false,
		CreatedAt:        time.Now().UTC(),
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ResetPassword_TokenNotFound(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	resetID, _ := uuid.NewV7()
	appID, _ := uuid.NewV7()

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetID).
		Return((*entities.PasswordResetToken)(nil), nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetID,
		PasswordResetToken: "any-token",
		NewPassword:        "NewPassword123!",
		ApplicationID:      appID,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrPasswordResetNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ResetPassword_TokenExpired(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	resetToken := newValidResetToken(userID)
	resetToken.ExpiresAt = time.Now().UTC().Add(-5 * time.Minute) // expired

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetToken.ID).Return(resetToken, nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetToken.ID,
		PasswordResetToken: resetToken.Token,
		NewPassword:        "NewPassword123!",
		ApplicationID:      appID,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrPasswordResetTokenExpired", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ResetPassword_TokenMismatch(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	resetToken := newValidResetToken(userID)

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetToken.ID).Return(resetToken, nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetToken.ID,
		PasswordResetToken: "wrong-token-value",
		NewPassword:        "NewPassword123!",
		ApplicationID:      appID,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrPasswordResetTokenMismatch", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ResetPassword_UserNotFound(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	resetToken := newValidResetToken(userID)

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetToken.ID).Return(resetToken, nil)
	repo.On("GetUserByID", mock.Anything, userID).Return((*entities.ApplicationUser)(nil), nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetToken.ID,
		PasswordResetToken: resetToken.Token,
		NewPassword:        "NewPassword123!",
		ApplicationID:      appID,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ResetPassword_ApplicationNotFound(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	appID, _ := uuid.NewV7()
	user := newResetUser(appID)
	resetToken := newValidResetToken(user.ID)

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetToken.ID).Return(resetToken, nil)
	repo.On("GetUserByID", mock.Anything, user.ID).Return(user, nil)
	repo.On("GetApplicationByID", mock.Anything, appID).Return((*entities.Application)(nil), nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetToken.ID,
		PasswordResetToken: resetToken.Token,
		NewPassword:        "NewPassword123!",
		ApplicationID:      appID,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrApplicationNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ResetPassword_Success(t *testing.T) {
	repo := new(mockResetPasswordRepo)
	orgID, _ := uuid.NewV7()
	app := newResetApp(orgID)
	user := newResetUser(app.ID)
	resetToken := newValidResetToken(user.ID)
	creds := newResetCredentials(user.ID)

	repo.On("GetPasswordResetByTokenID", mock.Anything, resetToken.ID).Return(resetToken, nil)
	repo.On("GetUserByID", mock.Anything, user.ID).Return(user, nil)
	repo.On("GetApplicationByID", mock.Anything, app.ID).Return(app, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("UpdateUser", mock.Anything, mock.AnythingOfType("*entities.ApplicationUser")).Return(user, nil)
	repo.On("UpdateUserCredentials", mock.Anything, mock.AnythingOfType("*entities.UserCredentials")).Return(nil)
	repo.On("RevokeRefreshTokenFromUser", mock.Anything, user.ID).Return(nil)
	repo.On("DeletePasswordResetFromUser", mock.Anything, user.ID).Return(nil)

	h := &Handler{repository: repo}
	err := h.Handler(context.Background(), Command{
		PasswordResetId:    resetToken.ID,
		PasswordResetToken: resetToken.Token,
		NewPassword:        "NewPassword123!",
		ApplicationID:      app.ID,
	})

	require.NoError(t, err)
	// Verify credentials were updated
	assert.False(t, creds.ShouldChangePass)
	assert.NotEqual(t, "old-password-hash", creds.PasswordHash)
	repo.AssertExpectations(t)
}
