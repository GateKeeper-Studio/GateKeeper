package confirmuseremail

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

type mockConfirmEmailRepo struct{ mock.Mock }

func (m *mockConfirmEmailRepo) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.TenantUser, error) {
	args := m.Called(ctx, email, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.TenantUser), args.Error(1)
}

func (m *mockConfirmEmailRepo) UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	return m.Called(ctx, emailConfirmation).Error(0)
}

func (m *mockConfirmEmailRepo) UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error) {
	args := m.Called(ctx, user)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.TenantUser), args.Error(1)
}

func (m *mockConfirmEmailRepo) AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error {
	return m.Called(ctx, authorizationCode).Error(0)
}

func (m *mockConfirmEmailRepo) GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error) {
	args := m.Called(ctx, email, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.EmailConfirmation), args.Error(1)
}

// Compile-time check
var _ IRepository = (*mockConfirmEmailRepo)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newTestUser(appID uuid.UUID) *entities.TenantUser {
	id, _ := uuid.NewV7()
	return &entities.TenantUser{
		ID:               id,
		ApplicationID:    appID,
		Email:            "user@example.com",
		IsActive:         true,
		IsEmailConfirmed: false,
		CreatedAt:        time.Now().UTC(),
	}
}

func newValidEmailConfirmation(userID uuid.UUID) *entities.EmailConfirmation {
	id, _ := uuid.NewV7()
	return &entities.EmailConfirmation{
		ID:        id,
		UserID:    userID,
		Email:     "user@example.com",
		Token:     "123456",
		CreatedAt: time.Now().UTC(),
		CoolDown:  time.Now().UTC().Add(-5 * time.Minute),
		ExpiresAt: time.Now().UTC().Add(20 * time.Minute),
		IsUsed:    false,
	}
}

func baseConfirmEmailCommand(appID uuid.UUID) Command {
	return Command{
		Token:               "123456",
		Email:               "user@example.com",
		ApplicationID:       appID,
		CodeChallengeMethod: "S256",
		ResponseType:        "code",
		Scope:               "openid",
		State:               "random-state",
		CodeChallenge:       "test-challenge",
		RedirectUri:         "https://app.example.com/callback",
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ConfirmEmail_UserNotFound(t *testing.T) {
	repo := new(mockConfirmEmailRepo)
	appID, _ := uuid.NewV7()

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).
		Return((*entities.TenantUser)(nil), nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseConfirmEmailCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ConfirmEmail_TokenInvalid(t *testing.T) {
	repo := new(mockConfirmEmailRepo)
	appID, _ := uuid.NewV7()
	user := newTestUser(appID)
	emailConf := newValidEmailConfirmation(user.ID)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)
	repo.On("GetEmailConfirmationByEmail", mock.Anything, "user@example.com", user.ID).Return(emailConf, nil)

	cmd := baseConfirmEmailCommand(appID)
	cmd.Token = "wrong-token"

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), cmd)

	require.Error(t, err)
	assert.Equal(t, "ErrConfirmationTokenInvalid", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ConfirmEmail_TokenAlreadyUsed(t *testing.T) {
	repo := new(mockConfirmEmailRepo)
	appID, _ := uuid.NewV7()
	user := newTestUser(appID)
	emailConf := newValidEmailConfirmation(user.ID)
	emailConf.IsUsed = true

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)
	repo.On("GetEmailConfirmationByEmail", mock.Anything, "user@example.com", user.ID).Return(emailConf, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseConfirmEmailCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrConfirmationTokenAlreadyUsed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ConfirmEmail_TokenExpired(t *testing.T) {
	repo := new(mockConfirmEmailRepo)
	appID, _ := uuid.NewV7()
	user := newTestUser(appID)
	emailConf := newValidEmailConfirmation(user.ID)
	emailConf.ExpiresAt = time.Now().UTC().Add(-1 * time.Minute)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)
	repo.On("GetEmailConfirmationByEmail", mock.Anything, "user@example.com", user.ID).Return(emailConf, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), baseConfirmEmailCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrConfirmationTokenAlreadyExpired", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ConfirmEmail_Success(t *testing.T) {
	repo := new(mockConfirmEmailRepo)
	appID, _ := uuid.NewV7()
	user := newTestUser(appID)
	emailConf := newValidEmailConfirmation(user.ID)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("AddAuthorizationCode", mock.Anything, mock.AnythingOfType("*entities.ApplicationAuthorizationCode")).Return(nil)
	repo.On("GetEmailConfirmationByEmail", mock.Anything, "user@example.com", user.ID).Return(emailConf, nil)
	repo.On("UpdateUser", mock.Anything, mock.AnythingOfType("*entities.TenantUser")).Return(user, nil)
	repo.On("UpdateEmailConfirmation", mock.Anything, mock.AnythingOfType("*entities.EmailConfirmation")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), baseConfirmEmailCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEmpty(t, resp.AuthorizationCode)
	assert.Equal(t, "https://app.example.com/callback", resp.RedirectUri)
	assert.Equal(t, "random-state", resp.State)
	assert.Equal(t, "test-challenge", resp.CodeChallenge)
	assert.Equal(t, "S256", resp.CodeChallengeMethod)
	assert.Equal(t, "code", resp.ResponseType)
	assert.True(t, user.IsEmailConfirmed)
	assert.True(t, emailConf.IsUsed)
	repo.AssertExpectations(t)
}
