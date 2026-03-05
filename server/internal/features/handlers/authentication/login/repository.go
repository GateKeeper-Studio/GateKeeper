package login

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.TenantUser, error)
	RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error
	AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error
	AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	AddChangePasswordCode(ctx context.Context, changePasswordCode *entities.ChangePasswordCode) error
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaPasskeyCredentials, error)
	AddMfaPasskeySession(ctx context.Context, session *entities.MfaPasskeySession) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.MfaRepository
	repositories.ChangePasswordCodeRepository
	repositories.SessionRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository:        repositories.ApplicationRepository{Store: q},
		UserRepository:               repositories.UserRepository{Store: q},
		UserProfileRepository:        repositories.UserProfileRepository{Store: q},
		MfaRepository:                repositories.MfaRepository{Store: q},
		ChangePasswordCodeRepository: repositories.ChangePasswordCodeRepository{Store: q},
		SessionRepository:            repositories.SessionRepository{Store: q},
		UserCredentialsRepository:    repositories.UserCredentialsRepository{Store: q},
	}
}
