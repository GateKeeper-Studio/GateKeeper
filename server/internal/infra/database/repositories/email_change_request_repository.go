package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/jackc/pgx/v5/pgtype"
)

// IEmailChangeRequestRepository defines all operations related to EmailChangeRequest entities.
type IEmailChangeRequestRepository interface {
	AddEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error
	GetEmailChangeRequestByToken(ctx context.Context, token string) (*entities.EmailChangeRequest, error)
	ConfirmEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error
	RevokeEmailChangeRequestsByUserID(ctx context.Context, req *entities.EmailChangeRequest) error
}

// EmailChangeRequestRepository is the shared implementation for EmailChangeRequest-related DB operations.
type EmailChangeRequestRepository struct {
	Store *pgstore.Queries
}

func (r EmailChangeRequestRepository) AddEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error {
	return r.Store.AddEmailChangeRequest(ctx, pgstore.AddEmailChangeRequestParams{
		ID:            req.ID,
		UserID:        req.UserID,
		ApplicationID: req.ApplicationID,
		NewEmail:      req.NewEmail,
		Token:         req.Token,
		CreatedAt:     pgtype.Timestamp{Time: req.CreatedAt, Valid: true},
		ExpiresAt:     pgtype.Timestamp{Time: req.ExpiresAt, Valid: true},
		IsConfirmed:   req.IsConfirmed,
	})
}

func (r EmailChangeRequestRepository) GetEmailChangeRequestByToken(ctx context.Context, token string) (*entities.EmailChangeRequest, error) {
	row, err := r.Store.GetEmailChangeRequestByToken(ctx, token)
	if err == ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	return &entities.EmailChangeRequest{
		ID:            row.ID,
		UserID:        row.UserID,
		ApplicationID: row.ApplicationID,
		NewEmail:      row.NewEmail,
		Token:         row.Token,
		CreatedAt:     row.CreatedAt.Time,
		ExpiresAt:     row.ExpiresAt.Time,
		IsConfirmed:   row.IsConfirmed,
	}, nil
}

func (r EmailChangeRequestRepository) ConfirmEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error {
	return r.Store.ConfirmEmailChangeRequest(ctx, req.ID)
}

func (r EmailChangeRequestRepository) RevokeEmailChangeRequestsByUserID(ctx context.Context, req *entities.EmailChangeRequest) error {
	return r.Store.RevokeEmailChangeRequestsByUserID(ctx, req.UserID)
}
