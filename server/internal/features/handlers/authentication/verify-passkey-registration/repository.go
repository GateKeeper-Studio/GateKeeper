package verifypasskeyregistration

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetMfaPasskeySessionByID(ctx context.Context, id uuid.UUID) (*entities.MfaPasskeySession, error)
	DeleteMfaPasskeySession(ctx context.Context, id uuid.UUID) error
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaPasskeyCredentials, error)
	AddWebAuthnCredential(ctx context.Context, cred *entities.MfaPasskeyCredentials) error
	EnableMfaMethod(ctx context.Context, methodID uuid.UUID) error
}

type Repository struct {
	repositories.MfaRepository
	repositories.UserRepository
	repositories.UserProfileRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		MfaRepository:         repositories.MfaRepository{Store: q},
		UserRepository:        repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
	}
}
