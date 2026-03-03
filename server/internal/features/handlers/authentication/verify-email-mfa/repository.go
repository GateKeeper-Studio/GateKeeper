package verifyemailmfa

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	DeleteEmailMfaCodeByID(ctx context.Context, emailMfaCodeID uuid.UUID) error
	GetMfaEmailCodeByToken(ctx context.Context, mfaMethodID uuid.UUID, token string) (*entities.MfaEmailCode, error)
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
}

type Repository struct {
	repositories.SessionRepository
	repositories.MfaRepository
	repositories.UserRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		SessionRepository: repositories.SessionRepository{Store: q},
		MfaRepository: repositories.MfaRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
	}
}
