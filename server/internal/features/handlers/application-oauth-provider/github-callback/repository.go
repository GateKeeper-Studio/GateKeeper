package githubcallback

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error)
	GetExternalOAuthStateByState(ctx context.Context, state string) (*entities.ExternalOAuthState, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	AddUser(ctx context.Context, newUser *entities.ApplicationUser) error
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error {
	err := r.Store.RemoveAuthorizationCode(ctx, pgstore.RemoveAuthorizationCodeParams{
		ApplicationID: applicationID,
		UserID:        userID,
	})

	return err
}

func (r Repository) AddAuthorizationCode(ctx context.Context, newAuthorizationCode *entities.ApplicationAuthorizationCode) error {
	err := r.Store.AddAuthorizationCode(ctx, pgstore.AddAuthorizationCodeParams{
		ID:                  newAuthorizationCode.ID,
		ApplicationID:       newAuthorizationCode.ApplicationID,
		UserID:              newAuthorizationCode.ApplicationUserId,
		ExpiredAt:           pgtype.Timestamp{Time: newAuthorizationCode.ExpiresAt, Valid: true},
		Code:                newAuthorizationCode.Code,
		RedirectUri:         newAuthorizationCode.RedirectUri,
		CodeChallenge:       newAuthorizationCode.CodeChallenge,
		CodeChallengeMethod: newAuthorizationCode.CodeChallengeMethod,
	})

	return err
}

func (r Repository) AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error {
	err := r.Store.AddExternalIdentity(ctx, pgstore.AddExternalIdentityParams{
		ID:                         newExternalIdentity.ID,
		UserID:                     newExternalIdentity.UserID,
		Provider:                   newExternalIdentity.Provider,
		ProviderUserID:             newExternalIdentity.ProviderUserID,
		ApplicationOauthProviderID: newExternalIdentity.ApplicationOAuthProviderID,
		CreatedAt:                  pgtype.Timestamp{Time: newExternalIdentity.CreatedAt, Valid: true},
		Email:                      newExternalIdentity.Email,
	})

	return err
}

func (r Repository) GetExternalOAuthStateByState(ctx context.Context, providerState string) (*entities.ExternalOAuthState, error) {
	oauthState, err := r.Store.GetExternalOAuthStateByState(ctx, providerState)

	if err != nil {
		return nil, err
	}

	oauthStateEntity := &entities.ExternalOAuthState{
		ID:                         oauthState.ID,
		ProviderState:              oauthState.ProviderState,
		ClientState:                *oauthState.ClientState,
		ClientCodeChallengeMethod:  *oauthState.ClientCodeChallengeMethod,
		ClientCodeChallenge:        *oauthState.ClientCodeChallenge,
		ClientScope:                *oauthState.ClientScope,
		ClientResponseType:         *oauthState.ClientResponseType,
		ClientRedirectUri:          *oauthState.ClientRedirectUri,
		ApplicationOAuthProviderID: oauthState.ApplicationOauthProviderID,
		CreatedAt:                  oauthState.CreatedAt.Time,
	}

	return oauthStateEntity, nil
}

func (r Repository) GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error) {
	oauthProvider, err := r.Store.GetApplicationOauthProviderByID(ctx, applicationOauthProviderID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	oauthProviderEntity := &entities.ApplicationOAuthProvider{
		ID:            oauthProvider.ID,
		Name:          oauthProvider.Name,
		Enabled:       oauthProvider.Enabled,
		ApplicationID: oauthProvider.ApplicationID,
		CreatedAt:     oauthProvider.CreatedAt.Time,
		UpdatedAt:     oauthProvider.UpdatedAt,
		ClientID:      oauthProvider.ClientID,
		ClientSecret:  oauthProvider.ClientSecret,
		RedirectURI:   oauthProvider.RedirectUri,
	}

	return oauthProviderEntity, nil
}

func (r Repository) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

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

func (r Repository) AddUser(ctx context.Context, newUser *entities.ApplicationUser) error {
	err := r.Store.AddUser(ctx, pgstore.AddUserParams{
		ID:               newUser.ID,
		Email:            newUser.Email,
		ApplicationID:    newUser.ApplicationID,
		CreatedAt:        pgtype.Timestamp{Time: newUser.CreatedAt, Valid: true},
		UpdatedAt:        newUser.UpdatedAt,
		IsActive:         newUser.IsActive,
		IsEmailConfirmed: newUser.IsEmailConfirmed,
	})

	return err
}

func (r Repository) AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	err := r.Store.AddUserProfile(ctx, pgstore.AddUserProfileParams{
		UserID:      newUserProfile.UserID,
		DisplayName: newUserProfile.DisplayName,
		FirstName:   newUserProfile.FirstName,
		LastName:    newUserProfile.LastName,
		Address:     newUserProfile.Address,
		PhoneNumber: newUserProfile.PhoneNumber,
		PhotoUrl:    newUserProfile.PhotoURL,
	})

	return err
}
