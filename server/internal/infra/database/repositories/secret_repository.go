package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// ISecretRepository defines all operations related to the ApplicationSecret entity.
type ISecretRepository interface {
	AddSecret(ctx context.Context, newSecret *entities.ApplicationSecret) error
	RemoveSecret(ctx context.Context, secretID uuid.UUID) error
	ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error)
}

// SecretRepository is the shared implementation for ApplicationSecret-related DB operations.
type SecretRepository struct {
	Store *pgstore.Queries
}

func (r SecretRepository) AddSecret(ctx context.Context, newSecret *entities.ApplicationSecret) error {
	return r.Store.AddSecret(ctx, pgstore.AddSecretParams{
		ID:            newSecret.ID,
		ApplicationID: newSecret.ApplicationID,
		Name:          newSecret.Name,
		Value:         newSecret.Value,
		CreatedAt:     pgtype.Timestamp{Time: newSecret.CreatedAt, Valid: true},
		UpdatedAt:     newSecret.UpdatedAt,
		ExpiresAt:     newSecret.ExpiresAt,
	})
}

func (r SecretRepository) RemoveSecret(ctx context.Context, secretID uuid.UUID) error {
	return r.Store.RemoveSecret(ctx, secretID)
}

func (r SecretRepository) ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error) {
	secrets, err := r.Store.ListSecretsFromApplication(ctx, applicationID)

	if err != nil && err != ErrNoRows {
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
