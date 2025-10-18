package getproviderdatabyid

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
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, request Query) (*Response, error) {
	oauthProvider, err := s.repository.GetApplicationOAuthProviderByID(ctx, request.ApplicaionOAuthProviderID)

	if err != nil {
		return nil, err
	}

	if oauthProvider == nil {
		return nil, &errors.ErrOAuthProviderNotFound
	}

	return &Response{
		ID:            oauthProvider.ID,
		Name:          oauthProvider.Name,
		Enabled:       oauthProvider.Enabled,
		ApplicationID: oauthProvider.ApplicationID,
		CreatedAt:     oauthProvider.CreatedAt,
		UpdatedAt:     oauthProvider.UpdatedAt,
		ClientID:      oauthProvider.ClientID,
		ClientSecret:  oauthProvider.ClientSecret,
		RedirectURI:   oauthProvider.RedirectURI,
	}, nil
}
