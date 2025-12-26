package signincredential

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error)
	RemoveAuthorizationCode(ctx context.Context, userID, applicationId uuid.UUID) error
	GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	AddRefreshToken(ctx context.Context, refreshToken *entities.RefreshToken) (*entities.RefreshToken, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) AddRefreshToken(ctx context.Context, refreshToken *entities.RefreshToken) (*entities.RefreshToken, error) {
	err := r.Store.AddRefreshToken(ctx, pgstore.AddRefreshTokenParams{
		UserID:             refreshToken.UserID,
		ID:                 refreshToken.ID,
		AvailableRefreshes: int32(refreshToken.AvailableRefreshes),
		ExpiresAt:          pgtype.Timestamp{Time: refreshToken.ExpiresAt, Valid: true},
		CreatedAt:          pgtype.Timestamp{Time: refreshToken.CreatedAt, Valid: true},
	})

	if err != nil {
		return nil, err
	}

	return &entities.RefreshToken{
		ID:                 refreshToken.ID,
		UserID:             refreshToken.UserID,
		AvailableRefreshes: refreshToken.AvailableRefreshes,
		ExpiresAt:          refreshToken.ExpiresAt,
		CreatedAt:          refreshToken.CreatedAt,
	}, nil
}

func (r Repository) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	err := r.Store.RevokeRefreshTokenFromUser(ctx, userID)

	if err != nil {
		return err
	}

	return nil
}

func (r Repository) GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error) {
	authorizationCode, err := r.Store.GetAuthorizationCodeById(ctx, code)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	return &entities.ApplicationAuthorizationCode{
		ID:                  authorizationCode.ID,
		ApplicationID:       authorizationCode.ApplicationID,
		ExpiresAt:           authorizationCode.ExpiredAt.Time,
		Code:                authorizationCode.Code,
		ApplicationUserId:   authorizationCode.UserID,
		RedirectUri:         authorizationCode.RedirectUri,
		CodeChallenge:       authorizationCode.CodeChallenge,
		CodeChallengeMethod: authorizationCode.CodeChallengeMethod,
	}, nil
}

func (r Repository) RemoveAuthorizationCode(ctx context.Context, userID, applicationId uuid.UUID) error {
	err := r.Store.RemoveAuthorizationCode(ctx, pgstore.RemoveAuthorizationCodeParams{
		ApplicationID: applicationId,
		UserID:        userID,
	})

	return err
}

func (r Repository) ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error) {
	secrets, err := r.Store.ListSecretsFromApplication(ctx, applicationID)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	var applicationSecrets []entities.ApplicationSecret

	for _, secret := range secrets {
		applicationSecrets = append(applicationSecrets, entities.ApplicationSecret{
			ID:            secret.ID,
			ApplicationID: secret.ApplicationID,
			Name:          secret.Name,
			Value:         secret.Value,
			CreatedAt:     secret.CreatedAt.Time,
			UpdatedAt:     secret.UpdatedAt,
			ExpiresAt:     secret.ExpiresAt,
		})
	}

	return &applicationSecrets, nil
}

func (r Repository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	userProfile, err := r.Store.GetUserProfileByUserId(ctx, userID)

	if err != nil {
		return nil, err
	}

	return &entities.UserProfile{
		UserID:      userProfile.UserID,
		DisplayName: userProfile.DisplayName,
		FirstName:   userProfile.FirstName,
		LastName:    userProfile.LastName,
		Address:     userProfile.Address,
		PhoneNumber: userProfile.PhoneNumber,
		PhotoURL:    userProfile.PhotoUrl,
	}, nil
}

func (r Repository) GetUserByID(ctx context.Context, id uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, id)

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}
