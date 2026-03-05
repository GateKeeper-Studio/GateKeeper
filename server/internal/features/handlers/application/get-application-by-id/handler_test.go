package getapplicationbyid

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

type mockGetAppRepo struct{ mock.Mock }

func (m *mockGetAppRepo) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.Application), args.Error(1)
}

func (m *mockGetAppRepo) ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error) {
	args := m.Called(ctx, applicationID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*[]entities.ApplicationSecret), args.Error(1)
}

var _ IRepository = (*mockGetAppRepo)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func newTestApp(appID uuid.UUID) *entities.Application {
	orgID, _ := uuid.NewV7()
	return &entities.Application{
		ID:                  appID,
		TenantID:            orgID,
		Name:                "Test App",
		IsActive:            true,
		PasswordHashSecret:  "secret-key-for-hashing",
		Badges:              []string{"badge1"},
		RefreshTokenTTLDays: 7,
		CreatedAt:           time.Now().UTC(),
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_GetApplicationByID_NotFound(t *testing.T) {
	repo := new(mockGetAppRepo)
	appID, _ := uuid.NewV7()

	repo.On("GetApplicationByID", mock.Anything, appID).
		Return((*entities.Application)(nil), nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{ApplicationID: appID})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "ErrApplicationNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_GetApplicationByID_Success(t *testing.T) {
	repo := new(mockGetAppRepo)
	appID, _ := uuid.NewV7()
	app := newTestApp(appID)

	secrets := []entities.ApplicationSecret{
		{
			ID:    uuid.New(),
			Name:  "api-key",
			Value: "abcdef1234567890",
		},
	}

	repo.On("GetApplicationByID", mock.Anything, appID).Return(app, nil)
	repo.On("ListSecretsFromApplication", mock.Anything, appID).Return(&secrets, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{ApplicationID: appID})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, appID, resp.ID)
	assert.Equal(t, "Test App", resp.Name)
	assert.True(t, resp.IsActive)
	assert.Len(t, resp.Secrets, 1)
	// Verify secret value is masked
	assert.Contains(t, resp.Secrets[0].Value, "****************")
	repo.AssertExpectations(t)
}

func TestHandler_GetApplicationByID_NoSecrets(t *testing.T) {
	repo := new(mockGetAppRepo)
	appID, _ := uuid.NewV7()
	app := newTestApp(appID)

	repo.On("GetApplicationByID", mock.Anything, appID).Return(app, nil)
	repo.On("ListSecretsFromApplication", mock.Anything, appID).
		Return((*[]entities.ApplicationSecret)(nil), nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{ApplicationID: appID})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, appID, resp.ID)
	assert.Empty(t, resp.Secrets)
	repo.AssertExpectations(t)
}
