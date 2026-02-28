package authorize

import (
	"context"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockAuthorizeRepo struct{ mock.Mock }

func (m *mockAuthorizeRepo) GetUserByEmail(ctx context.Context, email string, appID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, email, appID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockAuthorizeRepo) GetAuthorizationSession(ctx context.Context, userID uuid.UUID, token string) (*entities.SessionCode, error) {
	args := m.Called(ctx, userID, token)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.SessionCode), args.Error(1)
}

func (m *mockAuthorizeRepo) DeleteSessionCodeByID(ctx context.Context, id uuid.UUID) error {
	return m.Called(ctx, id).Error(0)
}

func (m *mockAuthorizeRepo) RemoveAuthorizationCode(ctx context.Context, userID, appID uuid.UUID) error {
	return m.Called(ctx, userID, appID).Error(0)
}

func (m *mockAuthorizeRepo) AddAuthorizationCode(ctx context.Context, code *entities.ApplicationAuthorizationCode) error {
	return m.Called(ctx, code).Error(0)
}

func (m *mockAuthorizeRepo) GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error) {
	args := m.Called(ctx, id)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.MfaTotpCode), args.Error(1)
}

func (m *mockAuthorizeRepo) DeleteMfaTotpCodeByID(ctx context.Context, id uuid.UUID) error {
	return m.Called(ctx, id).Error(0)
}

func (m *mockAuthorizeRepo) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserCredentials), args.Error(1)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newUser(appID uuid.UUID, active, confirmed bool) *entities.ApplicationUser {
	id, _ := uuid.NewV7()
	return &entities.ApplicationUser{
		ID:               id,
		ApplicationID:    appID,
		Email:            "user@example.com",
		IsActive:         active,
		IsEmailConfirmed: confirmed,
		CreatedAt:        time.Now().UTC(),
	}
}

func newCreds(userID uuid.UUID, shouldChange bool) *entities.UserCredentials {
	return &entities.UserCredentials{
		UserID:           userID,
		ShouldChangePass: shouldChange,
		CreatedAt:        time.Now().UTC(),
	}
}

func validSessionCode(userID uuid.UUID) *entities.SessionCode {
	id, _ := uuid.NewV7()
	return &entities.SessionCode{
		ID:        id,
		UserID:    userID,
		Token:     "valid-session-token",
		ExpiresAt: time.Now().UTC().Add(15 * time.Minute),
		CreatedAt: time.Now().UTC(),
	}
}

func baseAuthorizeCommand(appID uuid.UUID) Command {
	return Command{
		ApplicationID:       appID,
		Email:               "user@example.com",
		SessionCode:         "valid-session-token",
		CodeChallenge:       "test-challenge",
		CodeChallengeMethod: "S256",
		RedirectUri:         "https://app.example.com/callback",
		ResponseType:        "code",
		State:               "random-state",
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_Authorize_UserNotFound(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).
		Return((*entities.ApplicationUser)(nil), nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_UserNotActive(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, false, true)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotActive", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_EmailNotConfirmed(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, false)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrEmailNotConfirmed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_ShouldChangePassword(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	creds := newCreds(user.ID, true)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserShouldChangePassword", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_SessionCodeNotFound(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	creds := newCreds(user.ID, false)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("GetAuthorizationSession", mock.Anything, user.ID, "valid-session-token").
		Return((*entities.SessionCode)(nil), nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrSessionCodeNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_SessionCodeExpired(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	creds := newCreds(user.ID, false)

	expiredSession := validSessionCode(user.ID)
	expiredSession.ExpiresAt = time.Now().UTC().Add(-5 * time.Minute) // in the past

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("GetAuthorizationSession", mock.Anything, user.ID, "valid-session-token").Return(expiredSession, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrSessionCodeExpired", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_TOTP_MfaIDRequired(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	totp := constants.MfaMethodTotp
	user.Preferred2FAMethod = &totp
	creds := newCreds(user.ID, false)
	session := validSessionCode(user.ID)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("GetAuthorizationSession", mock.Anything, user.ID, "valid-session-token").Return(session, nil)

	cmd := baseAuthorizeCommand(appID)
	cmd.MfaID = nil // no MFA code provided

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), cmd)

	require.Error(t, err)
	assert.Equal(t, "ErrMfaCodeRequired", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_Success(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	creds := newCreds(user.ID, false)
	session := validSessionCode(user.ID)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("GetAuthorizationSession", mock.Anything, user.ID, "valid-session-token").Return(session, nil)
	repo.On("DeleteSessionCodeByID", mock.Anything, session.ID).Return(nil)
	repo.On("RemoveAuthorizationCode", mock.Anything, user.ID, appID).Return(nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), baseAuthorizeCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEmpty(t, resp.AuthorizationCode)
	assert.Equal(t, "https://app.example.com/callback", resp.RedirectUri)
	assert.Equal(t, "random-state", resp.State)
	assert.Equal(t, "test-challenge", resp.CodeChallenge)
	assert.Equal(t, "S256", resp.CodeChallengeMethod)
	repo.AssertExpectations(t)
}

func TestHandler_Authorize_Success_WithTOTP(t *testing.T) {
	repo := new(mockAuthorizeRepo)
	appID, _ := uuid.NewV7()
	user := newUser(appID, true, true)
	totp := constants.MfaMethodTotp
	user.Preferred2FAMethod = &totp
	creds := newCreds(user.ID, false)
	session := validSessionCode(user.ID)

	mfaCodeID, _ := uuid.NewV7()
	mfaCode := &entities.MfaTotpCode{
		ID:          mfaCodeID,
		MfaMethodID: uuid.Must(uuid.NewV7()),
		Secret:      "JBSWY3DPEHPK3PXP",
		CreatedAt:   time.Now().UTC(),
	}

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("GetAuthorizationSession", mock.Anything, user.ID, "valid-session-token").Return(session, nil)
	repo.On("GetMfaTotpCodeByID", mock.Anything, mfaCodeID).Return(mfaCode, nil)
	repo.On("DeleteMfaTotpCodeByID", mock.Anything, user.ID).Return(nil)
	repo.On("DeleteSessionCodeByID", mock.Anything, session.ID).Return(nil)
	repo.On("RemoveAuthorizationCode", mock.Anything, user.ID, appID).Return(nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)

	cmd := baseAuthorizeCommand(appID)
	cmd.MfaID = &mfaCodeID

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), cmd)

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEmpty(t, resp.AuthorizationCode)
	repo.AssertExpectations(t)
}
