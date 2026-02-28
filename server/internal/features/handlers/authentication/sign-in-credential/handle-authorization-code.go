package signincredential

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
)

func handleAuthorizationCode(ctx context.Context, handler *Handler, request Command) (*entities.ApplicationAuthorizationCode, error) {
	authorizationCode, err := handler.repository.GetAuthorizationCodeById(ctx, request.AuthorizationCode)

	if err != nil {
		return nil, err
	}

	if authorizationCode == nil {
		return nil, &errors.ErrAuthorizationCodeNotFound
	}

	currentDate := time.Now().UTC()

	if authorizationCode.ExpiresAt.Before(currentDate) {
		return nil, &errors.ErrAuthorizationCodeExpired
	}

	if authorizationCode.RedirectUri != request.RedirectURI {
		return nil, &errors.ErrAuthorizationCodeInvalidRedirectURI
	}

	if authorizationCode.ApplicationID != request.ClientID {
		return nil, &errors.ErrAuthorizationCodeInvalidClientID
	}

	return authorizationCode, nil
}
