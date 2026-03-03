package getprovidersdatabyapplicationid

import (
	"context"

	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, request Query) (*Response, error) {
	isApplicationExists, err := s.repository.CheckIfApplicationExists(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	if !isApplicationExists {
		return nil, &errors.ErrApplicationNotFound
	}

	providers, err := s.repository.GetApplicationOAuthProviders(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	response := make(Response, 0, len(*providers))

	for _, provider := range *providers {
		response = append(response, ApplicationProvider{
			ID:           provider.ID,
			Name:         provider.Name,
			IsEnabled:    provider.Enabled,
			ClientID:     provider.ClientID,
			ClientSecret: provider.ClientSecret,
			RedirectURI:  provider.RedirectURI,
			UpdatedAt:    provider.UpdatedAt,
			CreatedAt:    provider.CreatedAt,
		})
	}

	return &response, nil
}
