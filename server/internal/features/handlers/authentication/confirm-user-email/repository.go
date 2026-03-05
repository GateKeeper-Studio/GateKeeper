package confirmuseremail

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.TenantUser, error)
	UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
	GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error)
}

type Repository struct {
	repositories.UserRepository
	repositories.EmailConfirmationRepository
	repositories.AuthorizationCodeRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
		EmailConfirmationRepository: repositories.EmailConfirmationRepository{Store: q},
		AuthorizationCodeRepository: repositories.AuthorizationCodeRepository{Store: q},
	}
}
