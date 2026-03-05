package login

import (
	"context"
	"os"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	application_utils "github.com/gate-keeper/internal/features/utils"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// testPasswordHash holds an argon2 hash of "testpassword" computed once for the suite.
var testPasswordHash string

func TestMain(m *testing.M) {
	hash, err := application_utils.HashPassword("testpassword", "test-salt-key")
	if err != nil {
		panic("failed to pre-compute test password hash: " + err.Error())
	}
	testPasswordHash = hash
	os.Exit(m.Run())
}

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockLoginRepo struct{ mock.Mock }

func (m *mockLoginRepo) AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error {
	return m.Called(ctx, session).Error(0)
}

func (m *mockLoginRepo) GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error) {
	args := m.Called(ctx, mfaMethodID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).([]entities.MfaWebauthnCredentials), args.Error(1)
}

func (m *mockLoginRepo) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

func (m *mockLoginRepo) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserProfile), args.Error(1)
}

func (m *mockLoginRepo) GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.MfaUserSecret), args.Error(1)
}

func (m *mockLoginRepo) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.TenantUser, error) {
	args := m.Called(ctx, email, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.TenantUser), args.Error(1)
}

func (m *mockLoginRepo) RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockLoginRepo) AddMfaEmailCode(ctx context.Context, code *entities.MfaEmailCode) error {
	return m.Called(ctx, code).Error(0)
}

func (m *mockLoginRepo) AddMfaTotpCode(ctx context.Context, code *entities.MfaTotpCode) error {
	return m.Called(ctx, code).Error(0)
}

func (m *mockLoginRepo) AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error {
	return m.Called(ctx, sessionCode).Error(0)
}

func (m *mockLoginRepo) AddChangePasswordCode(ctx context.Context, code *entities.ChangePasswordCode) error {
	return m.Called(ctx, code).Error(0)
}

func (m *mockLoginRepo) GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error) {
	args := m.Called(ctx, userID, method)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.MfaMethod), args.Error(1)
}

func (m *mockLoginRepo) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserCredentials), args.Error(1)
}

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

// Compile-time check that mock satisfies the interface.
var _ mailservice.IMailService = (*mockMailService)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newActiveConfirmedUser(appID uuid.UUID) *entities.TenantUser {
	id, _ := uuid.NewV7()
	return &entities.TenantUser{
		ID:               id,
		ApplicationID:    appID,
		Email:            "user@example.com",
		IsActive:         true,
		IsEmailConfirmed: true,
		CreatedAt:        time.Now().UTC(),
	}
}

func newCredentials(userID uuid.UUID, shouldChange bool) *entities.UserCredentials {
	return &entities.UserCredentials{
		UserID:           userID,
		PasswordHash:     testPasswordHash,
		ShouldChangePass: shouldChange,
		CreatedAt:        time.Now().UTC(),
	}
}

func baseCommand(appID uuid.UUID) Command {
	return Command{
		ApplicationID: appID,
		Email:         "user@example.com",
		Password:      "testpassword",
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_Login_UserNotFound(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).
		Return((*entities.TenantUser)(nil), nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	_, err := h.Handler(context.Background(), baseCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Login_UserNotActive(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	user.IsActive = false

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	_, err := h.Handler(context.Background(), baseCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrUserNotActive", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Login_WrongPassword(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	creds := newCredentials(user.ID, false)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)

	cmd := baseCommand(appID)
	cmd.Password = "wrongpassword"

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	_, err := h.Handler(context.Background(), cmd)

	require.Error(t, err)
	assert.Equal(t, "ErrEmailOrPasswordInvalid", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Login_EmailNotConfirmed(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	user.IsEmailConfirmed = false
	creds := newCredentials(user.ID, false)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	_, err := h.Handler(context.Background(), baseCommand(appID))

	require.Error(t, err)
	assert.Equal(t, "ErrEmailNotConfirmed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_Login_Success(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	creds := newCredentials(user.ID, false)

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("RevokeAllChangePasswordCodeByUserID", mock.Anything, user.ID).Return(nil)
	repo.On("AddSessionCode", mock.Anything, mock.AnythingOfType("*entities.SessionCode")).Return(nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	resp, err := h.Handler(context.Background(), baseCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotNil(t, resp.SessionCode)
	assert.NotEmpty(t, *resp.SessionCode)
	assert.Nil(t, resp.MfaType)
	assert.Nil(t, resp.ChangePasswordCode)
	assert.Equal(t, user.ID, resp.UserID)
	repo.AssertExpectations(t)
}

func TestHandler_Login_Success_ShouldChangePassword(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	creds := newCredentials(user.ID, true) // force password change

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("RevokeAllChangePasswordCodeByUserID", mock.Anything, user.ID).Return(nil)
	repo.On("AddChangePasswordCode", mock.Anything, mock.AnythingOfType("*entities.ChangePasswordCode")).Return(nil)
	repo.On("AddSessionCode", mock.Anything, mock.AnythingOfType("*entities.SessionCode")).Return(nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	resp, err := h.Handler(context.Background(), baseCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotNil(t, resp.SessionCode)
	assert.NotNil(t, resp.ChangePasswordCode)
	assert.NotEmpty(t, *resp.ChangePasswordCode)
	repo.AssertExpectations(t)
}

func TestHandler_Login_MFA_TOTP(t *testing.T) {
	repo := new(mockLoginRepo)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	totpMethod := constants.MfaMethodTotp
	user.Preferred2FAMethod = &totpMethod
	creds := newCredentials(user.ID, false)

	mfaMethod := &entities.MfaMethod{
		ID:      uuid.Must(uuid.NewV7()),
		UserID:  user.ID,
		Type:    constants.MfaMethodTotp,
		Enabled: true,
	}
	mfaSecret := &entities.MfaUserSecret{
		ID:     uuid.Must(uuid.NewV7()),
		UserID: user.ID,
		Secret: "JBSWY3DPEHPK3PXP",
	}

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("RevokeAllChangePasswordCodeByUserID", mock.Anything, user.ID).Return(nil)
	repo.On("GetMfaMethodByUserID", mock.Anything, user.ID, constants.MfaMethodTotp).Return(mfaMethod, nil)
	repo.On("GetMfaTotpSecretValidationByUserID", mock.Anything, user.ID).Return(mfaSecret, nil)
	repo.On("AddMfaTotpCode", mock.Anything, mock.AnythingOfType("*entities.MfaTotpCode")).Return(nil)

	h := &Handler{repository: repo, mailService: new(mockMailService)}
	resp, err := h.Handler(context.Background(), baseCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotNil(t, resp.MfaType)
	assert.Equal(t, constants.MfaMethodTotp, *resp.MfaType)
	assert.NotNil(t, resp.MfaID)
	assert.Nil(t, resp.SessionCode)
	repo.AssertExpectations(t)
}

func TestHandler_Login_MFA_Email(t *testing.T) {
	repo := new(mockLoginRepo)
	mailSvc := new(mockMailService)
	appID, _ := uuid.NewV7()
	user := newActiveConfirmedUser(appID)
	emailMethod := constants.MfaMethodEmail
	user.Preferred2FAMethod = &emailMethod
	creds := newCredentials(user.ID, false)

	mfaMethod := &entities.MfaMethod{
		ID:      uuid.Must(uuid.NewV7()),
		UserID:  user.ID,
		Type:    constants.MfaMethodEmail,
		Enabled: true,
	}
	profile := &entities.UserProfile{
		UserID:    user.ID,
		FirstName: "Test",
		LastName:  "User",
	}

	repo.On("GetUserByEmail", mock.Anything, "user@example.com", appID).Return(user, nil)
	repo.On("GetUserCredentialsByUserID", mock.Anything, user.ID).Return(creds, nil)
	repo.On("RevokeAllChangePasswordCodeByUserID", mock.Anything, user.ID).Return(nil)
	repo.On("GetUserProfileByID", mock.Anything, user.ID).Return(profile, nil)
	repo.On("GetMfaMethodByUserID", mock.Anything, user.ID, constants.MfaMethodEmail).Return(mfaMethod, nil)
	repo.On("AddMfaEmailCode", mock.Anything, mock.AnythingOfType("*entities.MfaEmailCode")).Return(nil)
	// SendMfaEmail is called in a goroutine; mark it optional so AssertExpectations
	// does not fail if the goroutine hasn't finished before the assertion runs.
	mailSvc.On("SendMfaEmail", mock.Anything, mock.Anything, mock.Anything, mock.Anything).
		Maybe().Return(nil)

	h := &Handler{repository: repo, mailService: mailSvc}
	resp, err := h.Handler(context.Background(), baseCommand(appID))

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotNil(t, resp.MfaType)
	assert.Equal(t, constants.MfaMethodEmail, *resp.MfaType)
	assert.NotNil(t, resp.MfaID)
	assert.Nil(t, resp.SessionCode)
	repo.AssertExpectations(t)
}
