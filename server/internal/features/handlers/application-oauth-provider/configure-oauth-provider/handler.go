package configureoauthprovider

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, request Command) error {
	isApplicationExists, err := s.repository.CheckIfApplicationExists(ctx, request.ApplicationID)

	if err != nil {
		return err
	}

	if !isApplicationExists {
		return &errors.ErrApplicationNotFound
	}

	applicationOauthProvider, err := s.repository.GetApplicationOauthProviderByName(ctx, request.ApplicationID, request.Name)

	if err != nil {
		return err
	}

	if applicationOauthProvider != nil {
		applicationOauthProvider.ClientID = request.ClientID
		applicationOauthProvider.ClientSecret = request.ClientSecret
		applicationOauthProvider.RedirectURI = request.RedirectURI
		applicationOauthProvider.Enabled = request.Enabled

		err = s.repository.UpdateApplicationOauthProvider(ctx, applicationOauthProvider)

		if err != nil {
			return err
		}

		return nil
	}

	newApplicationOauthProvider := entities.NewApplicationOAuthProvider(
		request.ApplicationID,
		request.Name,
		request.ClientID,
		request.ClientSecret,
		request.RedirectURI,
		request.Enabled,
	)

	err = s.repository.AddApplicationOauthProvider(ctx, newApplicationOauthProvider)

	if err != nil {
		return err
	}

	return nil
}
