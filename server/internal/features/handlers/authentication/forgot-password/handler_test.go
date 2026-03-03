package forgotpassword

import (
	"context"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockForgotPasswordRepo struct{ mock.Mock }

func (m *mockForgotPasswordRepo) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockForgotPasswordRepo) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, email, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockForgotPasswordRepo) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserProfile), args.Error(1)
}

func (m *mockForgotPasswordRepo) CreatePasswordReset(ctx context.Context, passwordReset *entities.PasswordResetToken) error {
	return m.Called(ctx, passwordReset).Error(0)
}

func (m *mockForgotPasswordRepo) DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

// Compile-time check
var _ IRepository = (*mockForgotPasswordRepo)(nil)

// ---------------------------------------------------------------------------
// Mail service mock
// ---------------------------------------------------------------------------

type mockMailService struct{ mock.Mock }

func (m *mockMailService) SendEmailConfirmationEmail(ctx context.Context, to, userName, token string) error {
	return m.Called(ctx, to, userName, token).Error(0)
}

func (m *mockMailService) SendMfaEmail(ctx context.Context, to, userName, token string) error {
	return m.Called(ctx, to, userName, token).Error(0)
}

func (m *mockMailService) SendForgotPasswordEmail(ctx context.Context, to, userName, token string, passwordResetID, applicationID uuid.UUID) error {
	return m.Called(ctx, to, userName, token, passwordResetID, applicationID).Error(0)
}

var _ mailservice.IMailService = (*mockMailService)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newConfirmedUser(appID uuid.UUID) *entities.ApplicationUser {
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

func newTestProfile(userID uuid.UUID) *entities.UserProfile {
	return &entities.UserProfile{
		UserID:      userID,
		FirstName:   "Test",
		LastName:    "User",
		DisplayName: "Test User",
	}
}

func baseForgotPasswordCommand(appID uuid.UUID) Command {
	return Command{
		ApplicationID: appID,
		Email:         "user@example.com",
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ForgotPassword_UserNotFound(t *testing.T) {
	repo := new(mockForgotPasswordRepo)
	mailSvc := new(mockMailService)
	appID, _ := uuid.NewV7()

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).
		Return((*entities.ApplicationUser)(nil), nil)

	h := &Handler{repository: repo, mailService: mailSvc}
	err := h.Handler(context.Background(), baseForgotPasswordCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ForgotPassword_EmailNotConfirmed(t *testing.T) {
	repo := new(mockForgotPasswordRepo)
	mailSvc := new(mockMailService)
	appID, _ := uuid.NewV7()
	user := newConfirmedUser(appID)
	user.IsEmailConfirmed = false

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)

	h := &Handler{repository: repo, mailService: mailSvc}
	err := h.Handler(context.Background(), baseForgotPasswordCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrEmailNotConfirmed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ForgotPassword_Success(t *testing.T) {
	repo := new(mockForgotPasswordRepo)
	mailSvc := new(mockMailService)
	appID, _ := uuid.NewV7()
	user := newConfirmedUser(appID)
	profile := newTestProfile(user.ID)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("DeletePasswordResetFromUser", mock.Anything, user.ID).Return(nil)
	repo.On("CreatePasswordReset", mock.Anything, mock.AnythingOfType("*entities.PasswordResetToken")).Return(nil)
	repo.On("GetUserProfileByID", mock.Anything, user.ID).Return(profile, nil)
	mailSvc.On("SendForgotPasswordEmail", mock.Anything, mock.Anything, mock.Anything, mock.Anything, mock.Anything, mock.Anything).
		Maybe().Return(nil)

	h := &Handler{repository: repo, mailService: mailSvc}
	err := h.Handler(context.Background(), baseForgotPasswordCommand(appID))

	require.NoError(t, err)
	repo.AssertExpectations(t)
}
