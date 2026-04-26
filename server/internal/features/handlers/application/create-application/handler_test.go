package createapplication

import (
	"context"
	"fmt"
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

type mockCreateAppRepo struct{ mock.Mock }

func (m *mockCreateAppRepo) AddApplication(ctx context.Context, application *entities.Application) error {
	return m.Called(ctx, application).Error(0)
}

func (m *mockCreateAppRepo) AddRole(ctx context.Context, role *entities.ApplicationRole) error {
	return m.Called(ctx, role).Error(0)
}

var _ IRepository = (*mockCreateAppRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_CreateApplication_Success(t *testing.T) {
	repo := new(mockCreateAppRepo)
	orgID, _ := uuid.NewV7()
	desc := "Test description"

	repo.On("AddApplication", mock.Anything, mock.AnythingOfType("*entities.Application")).Return(nil)
	repo.On("AddRole", mock.Anything, mock.AnythingOfType("*entities.ApplicationRole")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:              "My App",
		Description:       &desc,
		Badges:            []string{"badge1"},
		HasMfaEmail:       false,
		HasMfaAuthApp:     false,
		HasMfaWebauthn:    false,
		TenantID:          orgID,
		CanSelfSignUp:     true,
		CanSelfForgotPass: true,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEqual(t, uuid.Nil, resp.ID)
	repo.AssertExpectations(t)
}

func TestHandler_CreateApplication_RepositoryError(t *testing.T) {
	repo := new(mockCreateAppRepo)
	orgID, _ := uuid.NewV7()

	repo.On("AddApplication", mock.Anything, mock.AnythingOfType("*entities.Application")).
		Return(fmt.Errorf("db error"))

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:     "My App",
		Badges:   []string{},
		TenantID: orgID,
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "db error", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_CreateApplication_CreatesDefaultRoles(t *testing.T) {
	repo := new(mockCreateAppRepo)
	orgID, _ := uuid.NewV7()

	repo.On("AddApplication", mock.Anything, mock.AnythingOfType("*entities.Application")).Return(nil)
	repo.On("AddRole", mock.Anything, mock.MatchedBy(func(role *entities.ApplicationRole) bool {
		return role.Name == "User" || role.Name == "Admin"
	})).Return(nil).Times(2)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:     "My App",
		Badges:   []string{},
		TenantID: orgID,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	// Verify exactly 2 AddRole calls (User + Admin)
	repo.AssertNumberOfCalls(t, "AddRole", 2)
	repo.AssertExpectations(t)

	_ = time.Now() // prevent unused import
}
