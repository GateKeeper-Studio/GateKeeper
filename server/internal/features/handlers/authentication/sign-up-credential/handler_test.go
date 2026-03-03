package signupcredential

import (
	"context"
	"fmt"
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

type mockSignUpRepo struct{ mock.Mock }

func (m *mockSignUpRepo) IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error) {
	args := m.Called(ctx, email, applicationID)
	return args.Bool(0), args.Error(1)
}

func (m *mockSignUpRepo) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

func (m *mockSignUpRepo) AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	return m.Called(ctx, newUserProfile).Error(0)
}

func (m *mockSignUpRepo) AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	return m.Called(ctx, emailConfirmation).Error(0)
}

func (m *mockSignUpRepo) AddUser(ctx context.Context, newUser *entities.ApplicationUser) error {
	return m.Called(ctx, newUser).Error(0)
}

func (m *mockSignUpRepo) AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	return m.Called(ctx, userCredentials).Error(0)
}

// Compile-time check
var _ IRepository = (*mockSignUpRepo)(nil)

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

// Compile-time check
var _ mailservice.IMailService = (*mockMailService)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newTestApplication(appID uuid.UUID) *entities.Application {
	orgID, _ := uuid.NewV7()
	return &entities.Application{
		ID:                  appID,
		OrganizationID:      orgID,
		Name:                "Test App",
		IsActive:            true,
		PasswordHashSecret:  "test-secret-key",
		RefreshTokenTTLDays: 7,
		CreatedAt:           time.Now().UTC(),
	}
}

func baseSignUpCommand(appID uuid.UUID) Command {
	return Command{
		ApplicationID: appID,
		FirstName:     "John",
		LastName:      "Doe",
		DisplayName:   "johndoe",
		Email:         "john@example.com",
		Password:      "SuperSecret123!",
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_SignUp_InvalidEmail(t *testing.T) {
	repo := new(mockSignUpRepo)
	mail := new(mockMailService)

	h := &Handler{repository: repo, mailService: mail}
	cmd := baseSignUpCommand(uuid.New())
	cmd.Email = "not-an-email"

	err := h.Handler(context.Background(), cmd)

	require.Error(t, err)
	assert.Equal(t, "ErrInvalidEmail", err.Error())
}

func TestHandler_SignUp_UserAlreadyExists(t *testing.T) {
	repo := new(mockSignUpRepo)
	mail := new(mockMailService)
	appID, _ := uuid.NewV7()

	repo.On("IsUserExistsByEmail", mock.Anything, "john@example.com", appID).
		Return(true, nil)

	h := &Handler{repository: repo, mailService: mail}
	err := h.Handler(context.Background(), baseSignUpCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserAlreadyExists", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignUp_ApplicationNotFound(t *testing.T) {
	repo := new(mockSignUpRepo)
	mail := new(mockMailService)
	appID, _ := uuid.NewV7()

	repo.On("IsUserExistsByEmail", mock.Anything, "john@example.com", appID).
		Return(false, nil)
	repo.On("GetApplicationByID", mock.Anything, appID).
		Return((*entities.Application)(nil), fmt.Errorf("application not found"))

	h := &Handler{repository: repo, mailService: mail}
	err := h.Handler(context.Background(), baseSignUpCommand(appID))

	require.Error(t, err)
	repo.AssertExpectations(t)
}

func TestHandler_SignUp_Success(t *testing.T) {
	repo := new(mockSignUpRepo)
	mail := new(mockMailService)
	appID, _ := uuid.NewV7()
	app := newTestApplication(appID)

	repo.On("IsUserExistsByEmail", mock.Anything, "john@example.com", appID).
		Return(false, nil)
	repo.On("GetApplicationByID", mock.Anything, appID).
		Return(app, nil)
	repo.On("AddUser", mock.Anything, mock.AnythingOfType("*entities.ApplicationUser")).
		Return(nil)
	repo.On("AddUserProfile", mock.Anything, mock.AnythingOfType("*entities.UserProfile")).
		Return(nil)
	repo.On("AddUserCredentials", mock.Anything, mock.AnythingOfType("*entities.UserCredentials")).
		Return(nil)
	repo.On("AddEmailConfirmation", mock.Anything, mock.AnythingOfType("*entities.EmailConfirmation")).
		Return(nil)
	mail.On("SendEmailConfirmationEmail", mock.Anything, "john@example.com", "John", mock.AnythingOfType("string")).
		Return(nil)

	h := &Handler{repository: repo, mailService: mail}
	err := h.Handler(context.Background(), baseSignUpCommand(appID))

	require.NoError(t, err)
	repo.AssertExpectations(t)
	// Note: mail is sent in a goroutine, so we don't assert it immediately
}
