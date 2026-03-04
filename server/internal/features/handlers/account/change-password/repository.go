package accountchangepassword

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
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	RevokeAllUserSessions(ctx context.Context, userID uuid.UUID) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserRepository
	repositories.UserCredentialsRepository
	repositories.ApplicationRepository
	repositories.RefreshTokenRepository
	repositories.UserSessionRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:            repositories.UserRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
		ApplicationRepository:     repositories.ApplicationRepository{Store: q},
		RefreshTokenRepository:    repositories.RefreshTokenRepository{Store: q},
		UserSessionRepository:     repositories.UserSessionRepository{Store: q},
		AuditLogRepository:        repositories.AuditLogRepository{Store: q},
	}
}
