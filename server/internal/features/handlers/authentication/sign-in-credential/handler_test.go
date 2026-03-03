package signincredential

import (
	"context"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockSignInRepo struct{ mock.Mock }

func (m *mockSignInRepo) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockSignInRepo) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.UserProfile), args.Error(1)
}

func (m *mockSignInRepo) ListSecretsFromApplication(ctx context.Context, appID uuid.UUID) (*[]entities.ApplicationSecret, error) {
	args := m.Called(ctx, appID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*[]entities.ApplicationSecret), args.Error(1)
}

func (m *mockSignInRepo) RemoveAuthorizationCode(ctx context.Context, userID, appID uuid.UUID) error {
	return m.Called(ctx, userID, appID).Error(0)
}

func (m *mockSignInRepo) GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error) {
	args := m.Called(ctx, code)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationAuthorizationCode), args.Error(1)
}

func (m *mockSignInRepo) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockSignInRepo) AddRefreshToken(ctx context.Context, rt *entities.RefreshToken) (*entities.RefreshToken, error) {
	args := m.Called(ctx, rt)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.RefreshToken), args.Error(1)
}

func (m *mockSignInRepo) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const (
	testRedirectURI  = "https://app.example.com/callback"
	testClientSecret = "super-secret-value"
)

// newValidAuthCode creates an authorization code that is fresh, not expired,
// and pre-loaded with a real PKCE challenge derived from verifier.
func newValidAuthCode(appID, userID uuid.UUID, verifier string) *entities.ApplicationAuthorizationCode {
	id, _ := uuid.NewV7()
	challenge := application_utils.GenerateCodeChallenge(verifier, "S256")
	return &entities.ApplicationAuthorizationCode{
		ID:                  id,
		ApplicationID:       appID,
		ApplicationUserId:   userID,
		RedirectUri:         testRedirectURI,
		CodeChallenge:       challenge,
		CodeChallengeMethod: "S256",
		ExpiresAt:           time.Now().UTC().Add(5 * time.Minute),
	}
}

func newTestSecrets(appID uuid.UUID) *[]entities.ApplicationSecret {
	return &[]entities.ApplicationSecret{
		{
			ID:            uuid.Must(uuid.NewV7()),
			ApplicationID: appID,
			Name:          "default",
			Value:         testClientSecret,
			CreatedAt:     time.Now().UTC(),
			ExpiresAt:     nil,
		},
	}
}

func newTestAppUser(appID uuid.UUID) *entities.ApplicationUser {
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

func newTestRefreshToken(userID uuid.UUID) *entities.RefreshToken {
	id, _ := uuid.NewV7()
	return &entities.RefreshToken{
		ID:        id,
		UserID:    userID,
		ExpiresAt: time.Now().UTC().Add(7 * 24 * time.Hour),
		CreatedAt: time.Now().UTC(),
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_SignIn_AuthCodeNotFound(t *testing.T) {
	repo := new(mockSignInRepo)
	codeID, _ := uuid.NewV7()
	appID, _ := uuid.NewV7()

	repo.On("GetAuthorizationCodeById", mock.Anything, codeID).
		Return((*entities.ApplicationAuthorizationCode)(nil), nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: codeID,
		ClientID:          appID,
		ClientSecret:      testClientSecret,
		CodeVerifier:      "any-verifier",
		RedirectURI:       testRedirectURI,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrAuthorizationCodeNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_AuthCodeExpired(t *testing.T) {
	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	verifier, _ := application_utils.GenerateCodeVerifier()

	authCode := newValidAuthCode(appID, userID, verifier)
	authCode.ExpiresAt = time.Now().UTC().Add(-1 * time.Minute) // expired

	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          appID,
		ClientSecret:      testClientSecret,
		CodeVerifier:      verifier,
		RedirectURI:       testRedirectURI,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrAuthorizationCodeExpired", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_InvalidRedirectURI(t *testing.T) {
	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	verifier, _ := application_utils.GenerateCodeVerifier()
	authCode := newValidAuthCode(appID, userID, verifier)

	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          appID,
		ClientSecret:      testClientSecret,
		CodeVerifier:      verifier,
		RedirectURI:       "https://evil.example.com/steal", // wrong URI
	})

	require.Error(t, err)
	assert.Equal(t, "ErrAuthorizationCodeInvalidRedirectURI", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_InvalidClientID(t *testing.T) {
	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	verifier, _ := application_utils.GenerateCodeVerifier()
	authCode := newValidAuthCode(appID, userID, verifier)

	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)

	wrongClientID, _ := uuid.NewV7()
	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          wrongClientID, // does not match authCode.ApplicationID
		ClientSecret:      testClientSecret,
		CodeVerifier:      verifier,
		RedirectURI:       testRedirectURI,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrAuthorizationCodeInvalidClientID", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_InvalidCodeChallenge(t *testing.T) {
	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	verifier, _ := application_utils.GenerateCodeVerifier()
	authCode := newValidAuthCode(appID, userID, verifier)

	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          appID,
		ClientSecret:      testClientSecret,
		CodeVerifier:      "wrong-verifier-that-does-not-match-challenge",
		RedirectURI:       testRedirectURI,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrInvalidCodeChallenge", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_InvalidClientSecret(t *testing.T) {
	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	userID, _ := uuid.NewV7()
	verifier, _ := application_utils.GenerateCodeVerifier()
	authCode := newValidAuthCode(appID, userID, verifier)

	// Return secrets that do NOT contain the provided client secret
	emptySecrets := &[]entities.ApplicationSecret{}
	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)
	repo.On("ListSecretsFromApplication", mock.Anything, appID).Return(emptySecrets, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          appID,
		ClientSecret:      "wrong-secret",
		CodeVerifier:      verifier,
		RedirectURI:       testRedirectURI,
	})

	require.Error(t, err)
	assert.Equal(t, "ErrInvalidClientSecret", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_SignIn_Success(t *testing.T) {
	// JWT_SECRET must be set for CreateToken to produce a valid token.
	t.Setenv("JWT_SECRET", "test-jwt-secret-for-unit-tests")

	repo := new(mockSignInRepo)
	appID, _ := uuid.NewV7()
	user := newTestAppUser(appID)
	verifier, _ := application_utils.GenerateCodeVerifier()
	authCode := newValidAuthCode(appID, user.ID, verifier)
	secrets := newTestSecrets(appID)
	profile := newTestProfile(user.ID)
	rt := newTestRefreshToken(user.ID)

	app := &entities.Application{
		ID:                  appID,
		Name:                "Test App",
		IsActive:            true,
		RefreshTokenTTLDays: 7,
	}

	repo.On("GetAuthorizationCodeById", mock.Anything, authCode.ID).Return(authCode, nil)
	repo.On("ListSecretsFromApplication", mock.Anything, appID).Return(secrets, nil)
	repo.On("RemoveAuthorizationCode", mock.Anything, user.ID, appID).Return(nil)
	repo.On("GetApplicationByID", mock.Anything, appID).Return(app, nil)
	repo.On("GetUserByID", mock.Anything, user.ID).Return(user, nil)
	repo.On("RevokeRefreshTokenFromUser", mock.Anything, user.ID).Return(nil)
	repo.On("AddRefreshToken", mock.Anything, mock.AnythingOfType("*entities.RefreshToken")).Return(rt, nil)
	repo.On("GetUserProfileByID", mock.Anything, user.ID).Return(profile, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		AuthorizationCode: authCode.ID,
		ClientID:          appID,
		ClientSecret:      testClientSecret,
		CodeVerifier:      verifier,
		RedirectURI:       testRedirectURI,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEmpty(t, resp.AccessToken)
	assert.NotEqual(t, uuid.Nil, resp.RefreshToken)
	assert.Equal(t, user.ID, resp.User.ID)
	assert.Equal(t, user.Email, resp.User.Email)
	assert.Equal(t, profile.FirstName, resp.User.FirstName)
	repo.AssertExpectations(t)
}
