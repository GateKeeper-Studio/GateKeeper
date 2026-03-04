package reauthenticate

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	AddStepUpToken(ctx context.Context, token *entities.StepUpToken) error
	RevokeStepUpTokensByUserID(ctx context.Context, userID uuid.UUID) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserRepository
	repositories.UserCredentialsRepository
	repositories.MfaRepository
	repositories.StepUpTokenRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:            repositories.UserRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
		MfaRepository:             repositories.MfaRepository{Store: q},
		StepUpTokenRepository:     repositories.StepUpTokenRepository{Store: q},
		AuditLogRepository:        repositories.AuditLogRepository{Store: q},
	}
}
