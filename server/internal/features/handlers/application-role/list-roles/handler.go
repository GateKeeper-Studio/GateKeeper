package listroles

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

	offset := (request.Page - 1) * request.PageSize

	response, err := s.repository.ListRolesFromApplicationPaged(ctx, request.ApplicationID, request.PageSize, offset)

	if err != nil {
		return nil, err
	}

	response.Page = request.Page
	response.PageSize = request.PageSize

	return response, nil
}
