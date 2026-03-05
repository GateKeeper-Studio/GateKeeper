package createtenant

import (
	"context"
	"fmt"
	"testing"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockCreateOrgRepo struct{ mock.Mock }

func (m *mockCreateOrgRepo) AddTenant(ctx context.Context, tenant *entities.Tenant) error {
	return m.Called(ctx, tenant).Error(0)
}

var _ IRepository = (*mockCreateOrgRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_CreateTenant_Success(t *testing.T) {
	repo := new(mockCreateOrgRepo)
	desc := "Test org description"

	repo.On("AddTenant", mock.Anything, mock.AnythingOfType("*entities.Tenant")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:        "My Tenant",
		Description: &desc,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, "My Tenant", resp.Name)
	assert.Equal(t, &desc, resp.Description)
	assert.NotEqual(t, uuid.Nil, resp.ID)
	assert.False(t, resp.CreatedAt.IsZero())
	repo.AssertExpectations(t)
}

func TestHandler_CreateTenant_NilDescription(t *testing.T) {
	repo := new(mockCreateOrgRepo)

	repo.On("AddTenant", mock.Anything, mock.AnythingOfType("*entities.Tenant")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:        "Org Without Desc",
		Description: nil,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Nil(t, resp.Description)
	repo.AssertExpectations(t)
}

func TestHandler_CreateTenant_RepositoryError(t *testing.T) {
	repo := new(mockCreateOrgRepo)

	repo.On("AddTenant", mock.Anything, mock.AnythingOfType("*entities.Tenant")).
		Return(fmt.Errorf("database connection refused"))

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name: "Failing Org",
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "database connection refused", err.Error())
	repo.AssertExpectations(t)
}
