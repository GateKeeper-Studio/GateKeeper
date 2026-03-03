package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/jackc/pgx/v5/pgtype"
)

// IExternalIdentityRepository defines all operations related to the ExternalIdentity entity.
type IExternalIdentityRepository interface {
	AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error
}

// ExternalIdentityRepository is the shared implementation for ExternalIdentity-related DB operations.
type ExternalIdentityRepository struct {
	Store *pgstore.Queries
}

func (r ExternalIdentityRepository) AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error {
	return r.Store.AddExternalIdentity(ctx, pgstore.AddExternalIdentityParams{
		ID:                         newExternalIdentity.ID,
		UserID:                     newExternalIdentity.UserID,
		Provider:                   newExternalIdentity.Provider,
		ProviderUserID:             newExternalIdentity.ProviderUserID,
		ApplicationOauthProviderID: newExternalIdentity.ApplicationOAuthProviderID,
		CreatedAt:                  pgtype.Timestamp{Time: newExternalIdentity.CreatedAt, Valid: true},
		Email:                      newExternalIdentity.Email,
	})
}
