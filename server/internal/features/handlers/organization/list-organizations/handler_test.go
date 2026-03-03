package listorganizations

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

type mockListOrgsRepo struct{ mock.Mock }

func (m *mockListOrgsRepo) ListOrganizations(ctx context.Context) (*[]entities.Organization, error) {
	args := m.Called(ctx)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*[]entities.Organization), args.Error(1)
}

var _ IRepository = (*mockListOrgsRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ListOrganizations_Success(t *testing.T) {
	repo := new(mockListOrgsRepo)

	orgs := []entities.Organization{
		{ID: uuid.New(), Name: "Org 1", CreatedAt: time.Now().UTC()},
		{ID: uuid.New(), Name: "Org 2", CreatedAt: time.Now().UTC()},
	}

	repo.On("ListOrganizations", mock.Anything).Return(&orgs, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{UserID: uuid.New()})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Len(t, *resp, 2)
	assert.Equal(t, "Org 1", (*resp)[0].Name)
	assert.Equal(t, "Org 2", (*resp)[1].Name)
	repo.AssertExpectations(t)
}

func TestHandler_ListOrganizations_Empty(t *testing.T) {
	repo := new(mockListOrgsRepo)

	orgs := []entities.Organization{}
	repo.On("ListOrganizations", mock.Anything).Return(&orgs, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{UserID: uuid.New()})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Empty(t, *resp)
	repo.AssertExpectations(t)
}

func TestHandler_ListOrganizations_RepositoryError(t *testing.T) {
	repo := new(mockListOrgsRepo)

	repo.On("ListOrganizations", mock.Anything).
		Return((*[]entities.Organization)(nil), fmt.Errorf("connection timeout"))

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{UserID: uuid.New()})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "connection timeout", err.Error())
	repo.AssertExpectations(t)
}
