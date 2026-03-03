package createorganization

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

func (m *mockCreateOrgRepo) AddOrganization(ctx context.Context, organization *entities.Organization) error {
	return m.Called(ctx, organization).Error(0)
}

var _ IRepository = (*mockCreateOrgRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_CreateOrganization_Success(t *testing.T) {
	repo := new(mockCreateOrgRepo)
	desc := "Test org description"

	repo.On("AddOrganization", mock.Anything, mock.AnythingOfType("*entities.Organization")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		Name:        "My Organization",
		Description: &desc,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, "My Organization", resp.Name)
	assert.Equal(t, &desc, resp.Description)
	assert.NotEqual(t, uuid.Nil, resp.ID)
	assert.False(t, resp.CreatedAt.IsZero())
	repo.AssertExpectations(t)
}

func TestHandler_CreateOrganization_NilDescription(t *testing.T) {
	repo := new(mockCreateOrgRepo)

	repo.On("AddOrganization", mock.Anything, mock.AnythingOfType("*entities.Organization")).Return(nil)

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

func TestHandler_CreateOrganization_RepositoryError(t *testing.T) {
	repo := new(mockCreateOrgRepo)

	repo.On("AddOrganization", mock.Anything, mock.AnythingOfType("*entities.Organization")).
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
