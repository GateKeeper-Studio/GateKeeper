package signincredential

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
)

func assignRefreshToken(ctx context.Context, handler *Handler, user entities.ApplicationUser, ttlDays int) (*entities.RefreshToken, error) {
	currentDate := time.Now().UTC()
	futureDate := currentDate.Add(time.Hour * 24 * time.Duration(ttlDays)).UTC()

	handler.repository.RevokeRefreshTokenFromUser(ctx, user.ID)
	refreshToken, err := entities.CreateRefreshToken(user.ID, futureDate)

	if err != nil {
		return nil, err
	}

	if _, err := handler.repository.AddRefreshToken(ctx, refreshToken); err != nil {
		return nil, err
	}

	return refreshToken, nil
}
