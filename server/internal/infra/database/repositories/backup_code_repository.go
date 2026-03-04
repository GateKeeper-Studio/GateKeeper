package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IBackupCodeRepository defines all operations related to BackupCode entities.
type IBackupCodeRepository interface {
	AddBackupCode(ctx context.Context, code *entities.BackupCode) error
	GetUnusedBackupCodesByUserID(ctx context.Context, userID uuid.UUID) ([]*entities.BackupCode, error)
	MarkBackupCodeUsed(ctx context.Context, codeID uuid.UUID) error
	DeleteBackupCodesByUserID(ctx context.Context, userID uuid.UUID) error
	CountUnusedBackupCodesByUserID(ctx context.Context, userID uuid.UUID) (int64, error)
}

// BackupCodeRepository is the shared implementation for BackupCode-related DB operations.
type BackupCodeRepository struct {
	Store *pgstore.Queries
}

func (r BackupCodeRepository) AddBackupCode(ctx context.Context, code *entities.BackupCode) error {
	return r.Store.AddBackupCode(ctx, pgstore.AddBackupCodeParams{
		ID:        code.ID,
		UserID:    code.UserID,
		CodeHash:  code.CodeHash,
		IsUsed:    code.IsUsed,
		CreatedAt: pgtype.Timestamp{Time: code.CreatedAt, Valid: true},
		UsedAt:    nil,
	})
}

func (r BackupCodeRepository) GetUnusedBackupCodesByUserID(ctx context.Context, userID uuid.UUID) ([]*entities.BackupCode, error) {
	rows, err := r.Store.GetUnusedBackupCodesByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}

	codes := make([]*entities.BackupCode, 0, len(rows))
	for _, row := range rows {
		codes = append(codes, &entities.BackupCode{
			ID:        row.ID,
			UserID:    row.UserID,
			CodeHash:  row.CodeHash,
			IsUsed:    row.IsUsed,
			CreatedAt: row.CreatedAt.Time,
			UsedAt:    row.UsedAt,
		})
	}

	return codes, nil
}

func (r BackupCodeRepository) MarkBackupCodeUsed(ctx context.Context, codeID uuid.UUID) error {
	return r.Store.MarkBackupCodeUsed(ctx, codeID)
}

func (r BackupCodeRepository) DeleteBackupCodesByUserID(ctx context.Context, userID uuid.UUID) error {
	return r.Store.DeleteBackupCodesByUserID(ctx, userID)
}

func (r BackupCodeRepository) CountUnusedBackupCodesByUserID(ctx context.Context, userID uuid.UUID) (int64, error) {
	return r.Store.CountUnusedBackupCodesByUserID(ctx, userID)
}
