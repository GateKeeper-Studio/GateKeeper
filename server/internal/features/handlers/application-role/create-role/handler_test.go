package createrole

import (
	"context"
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

type mockCreateRoleRepo struct{ mock.Mock }

func (m *mockCreateRoleRepo) AddRole(ctx context.Context, role *entities.ApplicationRole) error {
	return m.Called(ctx, role).Error(0)
}

func (m *mockCreateRoleRepo) CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error) {
	args := m.Called(ctx, applicationID)
	return args.Bool(0), args.Error(1)
}

// Compile-time check
var _ IRepository = (*mockCreateRoleRepo)(nil)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func strPtr(s string) *string { return &s }

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_CreateRole_ApplicationNotFound(t *testing.T) {
	repo := new(mockCreateRoleRepo)
	appID, _ := uuid.NewV7()

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(false, nil)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		ApplicationID: appID,
		Name:          "Editor",
		Description:   strPtr("Can edit content"),
	})

	require.Error(t, err)
	assert.Equal(t, "ErrApplicationNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_CreateRole_Success(t *testing.T) {
	repo := new(mockCreateRoleRepo)
	appID, _ := uuid.NewV7()

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("AddRole", mock.Anything, mock.AnythingOfType("*entities.ApplicationRole")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		ApplicationID: appID,
		Name:          "Editor",
		Description:   strPtr("Can edit content"),
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.NotEqual(t, uuid.Nil, resp.ID)
	assert.Equal(t, "Editor", resp.Name)
	assert.Equal(t, strPtr("Can edit content"), resp.Description)
	assert.Equal(t, appID, resp.ApplicationID)
	assert.False(t, resp.CreatedAt.IsZero())
	repo.AssertExpectations(t)
}

func TestHandler_CreateRole_NilDescription(t *testing.T) {
	repo := new(mockCreateRoleRepo)
	appID, _ := uuid.NewV7()

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("AddRole", mock.Anything, mock.AnythingOfType("*entities.ApplicationRole")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		ApplicationID: appID,
		Name:          "Viewer",
		Description:   nil,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, "Viewer", resp.Name)
	assert.Nil(t, resp.Description)
	repo.AssertExpectations(t)
}

func TestHandler_CreateRole_RepositoryError(t *testing.T) {
	repo := new(mockCreateRoleRepo)
	appID, _ := uuid.NewV7()

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("AddRole", mock.Anything, mock.AnythingOfType("*entities.ApplicationRole")).
		Return(assert.AnError)

	h := &Handler{repository: repo}
	_, err := h.Handler(context.Background(), Command{
		ApplicationID: appID,
		Name:          "Editor",
		Description:   strPtr("Can edit content"),
	})

	require.Error(t, err)
	repo.AssertExpectations(t)
}
