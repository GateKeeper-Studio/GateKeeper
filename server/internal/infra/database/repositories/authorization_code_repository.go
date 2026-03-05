package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IAuthorizationCodeRepository defines all operations related to the ApplicationAuthorizationCode entity.
type IAuthorizationCodeRepository interface {
	AddAuthorizationCode(ctx context.Context, newAuthorizationCode *entities.ApplicationAuthorizationCode) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error)
}

// AuthorizationCodeRepository is the shared implementation for AuthorizationCode-related DB operations.
type AuthorizationCodeRepository struct {
	Store *pgstore.Queries
}

func (r AuthorizationCodeRepository) AddAuthorizationCode(ctx context.Context, newAuthorizationCode *entities.ApplicationAuthorizationCode) error {
	return r.Store.AddAuthorizationCode(ctx, pgstore.AddAuthorizationCodeParams{
		ID:                  newAuthorizationCode.ID,
		ApplicationID:       newAuthorizationCode.ApplicationID,
		UserID:              newAuthorizationCode.TenantUserId,
		ExpiredAt:           pgtype.Timestamp{Time: newAuthorizationCode.ExpiresAt, Valid: true},
		Code:                newAuthorizationCode.Code,
		RedirectUri:         newAuthorizationCode.RedirectUri,
		CodeChallenge:       newAuthorizationCode.CodeChallenge,
		CodeChallengeMethod: newAuthorizationCode.CodeChallengeMethod,
		Nonce:               newAuthorizationCode.Nonce,
		Scope:               newAuthorizationCode.Scope,
	})
}

func (r AuthorizationCodeRepository) RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error {
	return r.Store.RemoveAuthorizationCode(ctx, pgstore.RemoveAuthorizationCodeParams{
		ApplicationID: applicationID,
		UserID:        userID,
	})
}

func (r AuthorizationCodeRepository) GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error) {
	authorizationCode, err := r.Store.GetAuthorizationCodeById(ctx, code)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	return &entities.ApplicationAuthorizationCode{
		ID:                  authorizationCode.ID,
		ApplicationID:       authorizationCode.ApplicationID,
		ExpiresAt:           authorizationCode.ExpiredAt.Time,
		Code:                authorizationCode.Code,
		TenantUserId:   authorizationCode.UserID,
		RedirectUri:         authorizationCode.RedirectUri,
		CodeChallenge:       authorizationCode.CodeChallenge,
		CodeChallengeMethod: authorizationCode.CodeChallengeMethod,
		Nonce:               authorizationCode.Nonce,
		Scope:               authorizationCode.Scope,
	}, nil
}
