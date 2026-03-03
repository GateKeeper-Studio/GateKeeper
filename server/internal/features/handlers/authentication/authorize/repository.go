package authorize

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	GetAuthorizationSession(ctx context.Context, userID uuid.UUID, sessionCodeToken string) (*entities.SessionCode, error)
	DeleteSessionCodeByID(ctx context.Context, sessionCodeID uuid.UUID) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
	GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error)
	DeleteMfaTotpCode(ctx context.Context, id uuid.UUID) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
}

type Repository struct {
	repositories.UserRepository
	repositories.SessionRepository
	repositories.AuthorizationCodeRepository
	repositories.MfaRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:              repositories.UserRepository{Store: q},
		SessionRepository:           repositories.SessionRepository{Store: q},
		AuthorizationCodeRepository: repositories.AuthorizationCodeRepository{Store: q},
		MfaRepository:               repositories.MfaRepository{Store: q},
		UserCredentialsRepository:   repositories.UserCredentialsRepository{Store: q},
	}
}
