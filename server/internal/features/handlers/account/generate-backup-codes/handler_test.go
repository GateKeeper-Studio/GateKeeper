package accountgeneratebackupcodes

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
	"github.com/stretchr/testify/require"
)

// ---------------------------------------------------------------------------
// Repository mock
// ---------------------------------------------------------------------------

type mockBackupCodesRepo struct{ mock.Mock }

func (m *mockBackupCodesRepo) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	args := m.Called(ctx, userID)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*entities.ApplicationUser), args.Error(1)
}

func (m *mockBackupCodesRepo) DeleteBackupCodesByUserID(ctx context.Context, userID uuid.UUID) error {
	return m.Called(ctx, userID).Error(0)
}

func (m *mockBackupCodesRepo) AddBackupCode(ctx context.Context, code *entities.BackupCode) error {
	return m.Called(ctx, code).Error(0)
}

func (m *mockBackupCodesRepo) AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error {
	return m.Called(ctx, auditLog).Error(0)
}

var _ IRepository = (*mockBackupCodesRepo)(nil)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHandler_GenerateBackupCodes_Success(t *testing.T) {
	repo := new(mockBackupCodesRepo)
	userID := uuid.New()
	appID := uuid.New()
	user := &entities.ApplicationUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("DeleteBackupCodesByUserID", mock.Anything, userID).Return(nil)
	repo.On("AddBackupCode", mock.Anything, mock.AnythingOfType("*entities.BackupCode")).Return(nil).Times(backupCodeCount)
	repo.On("AddAuditLog", mock.Anything, mock.AnythingOfType("*entities.AuditLog")).Return(nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:        userID,
		ApplicationID: appID,
	})

	require.NoError(t, err)
	require.NotNil(t, resp)
	assert.Len(t, resp.Codes, backupCodeCount)
	assert.Contains(t, resp.Message, "Backup codes generated successfully")

	// Verify each code is in XXXX-XXXX format
	for _, code := range resp.Codes {
		assert.Regexp(t, `^[A-Z2-9]{4}-[A-Z2-9]{4}$`, code)
	}

	repo.AssertExpectations(t)
}

func TestHandler_GenerateBackupCodes_UserNotFound(t *testing.T) {
	repo := new(mockBackupCodesRepo)
	userID := uuid.New()

	repo.On("GetUserByID", mock.Anything, userID).Return(nil, nil)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:        userID,
		ApplicationID: uuid.New(),
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, &errors.ErrUserNotFound, err)
	repo.AssertExpectations(t)
}

func TestHandler_GenerateBackupCodes_DeleteOldCodesError(t *testing.T) {
	repo := new(mockBackupCodesRepo)
	userID := uuid.New()
	user := &entities.ApplicationUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}
	dbErr := fmt.Errorf("delete failed")

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("DeleteBackupCodesByUserID", mock.Anything, userID).Return(dbErr)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:        userID,
		ApplicationID: uuid.New(),
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "delete failed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_GenerateBackupCodes_AddCodeError(t *testing.T) {
	repo := new(mockBackupCodesRepo)
	userID := uuid.New()
	user := &entities.ApplicationUser{
		ID:        userID,
		Email:     "user@test.com",
		IsActive:  true,
		CreatedAt: time.Now().UTC(),
	}
	dbErr := fmt.Errorf("insert failed")

	repo.On("GetUserByID", mock.Anything, userID).Return(user, nil)
	repo.On("DeleteBackupCodesByUserID", mock.Anything, userID).Return(nil)
	// First AddBackupCode call fails
	repo.On("AddBackupCode", mock.Anything, mock.AnythingOfType("*entities.BackupCode")).Return(dbErr).Once()

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:        userID,
		ApplicationID: uuid.New(),
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "insert failed", err.Error())
	repo.AssertExpectations(t)
}

func TestHandler_GenerateBackupCodes_GetUserDBError(t *testing.T) {
	repo := new(mockBackupCodesRepo)
	userID := uuid.New()
	dbErr := fmt.Errorf("connection refused")

	repo.On("GetUserByID", mock.Anything, userID).Return(nil, dbErr)

	h := &Handler{repository: repo}
	resp, err := h.Handler(context.Background(), Command{
		UserID:        userID,
		ApplicationID: uuid.New(),
	})

	require.Error(t, err)
	assert.Nil(t, resp)
	assert.Equal(t, "connection refused", err.Error())
	repo.AssertExpectations(t)
}
