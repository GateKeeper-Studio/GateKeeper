package listroles

import (
	"context"
	"fmt"
	"testing"

	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockListRolesRepo struct{ mock.Mock }

func (m *mockListRolesRepo) ListRolesFromApplicationPaged(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*Response, error) {
	args := m.Called(ctx, applicationID, limit, offset)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*Response), args.Error(1)
}

func (m *mockListRolesRepo) CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error) {
	args := m.Called(ctx, applicationID)
	return args.Bool(0), args.Error(1)
}

var _ IRepository = (*mockListRolesRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_ListRoles_ApplicationNotFound(t *testing.T) {
	repo := new(mockListRolesRepo)
	appID, _ := uuid.NewV7()

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(false, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{
		ApplicationID: appID,
		Page:          1,
		PageSize:      10,
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "ErrApplicationNotFound", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_ListRoles_Success(t *testing.T) {
	repo := new(mockListRolesRepo)
	appID, _ := uuid.NewV7()

	desc := "Admin role"
	expected := &Response{
		TotalCount: 2,
		Data: []RoleResponse{
			{ID: uuid.New(), Name: "Admin", Description: &desc},
			{ID: uuid.New(), Name: "User", Description: nil},
		},
	}

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("ListRolesFromApplicationPaged", mock.Anything, appID, 10, 0).Return(expected, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{
		ApplicationID: appID,
		Page:          1,
		PageSize:      10,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, 2, resp.TotalCount)
	assert.Len(t, resp.Data, 2)
	assert.Equal(t, 1, resp.Page)
	assert.Equal(t, 10, resp.PageSize)
	repo.AssertExpectations(t)
}

func TestHandler_ListRoles_EmptyResult(t *testing.T) {
	repo := new(mockListRolesRepo)
	appID, _ := uuid.NewV7()

	expected := &Response{
		TotalCount: 0,
		Data:       []RoleResponse{},
	}

	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("ListRolesFromApplicationPaged", mock.Anything, appID, 10, 0).Return(expected, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{
		ApplicationID: appID,
		Page:          1,
		PageSize:      10,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, 0, resp.TotalCount)
	assert.Empty(t, resp.Data)
	repo.AssertExpectations(t)
}

func TestHandler_ListRoles_Pagination(t *testing.T) {
	repo := new(mockListRolesRepo)
	appID, _ := uuid.NewV7()

	expected := &Response{
		TotalCount: 15,
		Data: []RoleResponse{
			{ID: uuid.New(), Name: "Role6"},
		},
	}

	// Page 2 with PageSize 5 => offset = (2-1)*5 = 5
	repo.On("CheckIfApplicationExists", mock.Anything, appID).Return(true, nil)
	repo.On("ListRolesFromApplicationPaged", mock.Anything, appID, 5, 5).Return(expected, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Query{
		ApplicationID: appID,
		Page:          2,
		PageSize:      5,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Equal(t, 15, resp.TotalCount)
	assert.Equal(t, 2, resp.Page)
	assert.Equal(t, 5, resp.PageSize)
	repo.AssertExpectations(t)

	_ = fmt.Sprint() // prevent unused import
}
