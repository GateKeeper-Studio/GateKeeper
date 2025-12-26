package generateauthappsecret

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/pquerna/otp"
	"github.com/pquerna/otp/totp"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	application, err := s.repository.GetApplicationByID(ctx, command.ApplicationID)

	if err != nil {
		return nil, err
	}

	if application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	user, err := s.repository.GetUserByID(ctx, command.UserID)

	if err != nil {
		return nil, err
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodTotp)

	if err != nil {
		return nil, err
	}

	if mfaMethod == nil {
		// Create a new TOTP MFA method if it doesn't exist and if the application has MFA Auth App enabled
		if application.HasMfaAuthApp {
			mfaMethod = entities.AddMfaMethod(user.ID, constants.MfaMethodTotp)

			err := s.repository.AddMfaMethod(ctx, mfaMethod)

			if err != nil {
				return nil, err
			}

		} else {
			return nil, &errors.ErrMfaAuthAppNotEnabled
		}
	}

	key, err := totp.Generate(totp.GenerateOpts{
		Issuer:      application.Name,
		AccountName: user.Email,
		Period:      30,
		SecretSize:  32,
		Secret:      []byte(entities.GenerateRandomString(16)),
		Digits:      6,
		Algorithm:   otp.AlgorithmSHA1,
		Rand:        nil,
	})

	if err != nil {
		return nil, err
	}

	// s.repository.RevokeTotpSecretsByUserID(ctx, user.ID) // Revoke existing TOTP secrets for the user

	secret := key.Secret()

	mfaUserSecret := entities.NewMfaUserSecret(user.ID, secret)

	// s.repository.RevokeMfaUserSecret(ctx, user.ID)
	if err := s.repository.AddMfaTotpSecretValidation(ctx, mfaUserSecret); err != nil {
		return nil, err
	}

	// user.TwoFactorSecret = &secret
	// s.repository.UpdateUser(ctx, user)

	return &Response{
		OtpUrl: key.URL(),
	}, nil
}
