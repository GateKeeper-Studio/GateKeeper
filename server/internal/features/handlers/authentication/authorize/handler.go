package authorize

import (
	"context"
	"strings"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	// Validate response_type (OAuth2 / OIDC)
	if command.ResponseType != "code" {
		return nil, &errors.ErrInvalidResponseType
	}

	// Validate code_challenge_method
	if command.CodeChallengeMethod != "S256" && command.CodeChallengeMethod != "plain" {
		return nil, &errors.ErrInvalidCodeChallengeMethod
	}

	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return nil, &errors.ErrUserNotFound
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	if !user.IsActive {
		return nil, &errors.ErrUserNotActive
	}

	if !user.IsEmailConfirmed {
		return nil, &errors.ErrEmailNotConfirmed
	}

	userCredentials, err := s.repository.GetUserCredentialsByUserID(ctx, user.ID)

	if err != nil {
		return nil, err
	}

	// If user has credentials and is required to change password, return error to prompt user to change password before authorizing
	if userCredentials != nil && userCredentials.ShouldChangePass {
		return nil, &errors.ErrUserShouldChangePassword
	}

	sessionCode, err := s.repository.GetAuthorizationSession(ctx, user.ID, command.SessionCode)

	if err != nil {
		return nil, err
	}

	if sessionCode == nil {
		return nil, &errors.ErrSessionCodeNotFound
	}

	if sessionCode.ExpiresAt.Before(time.Now().UTC()) {
		return nil, &errors.ErrSessionCodeExpired
	}

	if err := s.repository.DeleteSessionCodeByID(ctx, sessionCode.ID); err != nil {
		return nil, err
	}

	// Normalise and store scope
	scope := command.Scope
	if scope == "" {
		scope = "openid"
	}

	// Nonce is required when openid scope is requested (OIDC Core 1.0 §3.1.2.1)
	var nonce *string
	if strings.Contains(scope, "openid") && command.Nonce != "" {
		n := command.Nonce
		nonce = &n
	}

	authorizationCode, err := entities.CreateApplicationAuthorizationCode(
		command.ApplicationID,
		user.ID,
		command.RedirectUri,
		command.CodeChallenge,
		command.CodeChallengeMethod,
		nonce,
		&scope,
	)

	if err != nil {
		return nil, err
	}

	if err := s.repository.RemoveAuthorizationCode(ctx, user.ID, command.ApplicationID); err != nil {
		return nil, err
	}

	if err := s.repository.AddAuthorizationCode(ctx, authorizationCode); err != nil {
		return nil, err
	}

	return &Response{
		AuthorizationCode:   authorizationCode.ID.String(),
		RedirectUri:         command.RedirectUri,
		State:               command.State,
		CodeChallenge:       command.CodeChallenge,
		CodeChallengeMethod: command.CodeChallengeMethod,
		ResponseType:        command.ResponseType,
		Scope:               scope,
	}, nil
}
