package confirmmfaauthappsecret

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/pquerna/otp/totp"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) error {
	user, err := s.repository.GetUserByID(ctx, command.UserID)

	if err != nil {
		return err
	}

	if user == nil {
		return &errors.ErrUserNotFound
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodTotp)

	if err != nil {
		return err
	}

	if mfaMethod == nil {
		return &errors.ErrMfaAuthAppNotEnabled
	}

	mfaTotpSecretValidation, err := s.repository.GetMfaTotpSecretValidationByUserId(ctx, command.UserID)

	if err != nil {
		return err
	}

	if mfaTotpSecretValidation == nil {
		return &errors.ErrMfaUserSecretNotFound
	}

	if mfaTotpSecretValidation.IsValidated {
		return &errors.ErrMfaUserSecretAlreadyValidated
	}

	isValid := totp.Validate(command.MfaAuthAppCode, mfaTotpSecretValidation.Secret)

	if !isValid {
		return &errors.ErrInvalidMfaAuthAppCode
	}

	mfaTotpSecretValidation.Validate()

	if err := s.repository.UpdateMfaTotpSecretValidation(ctx, mfaTotpSecretValidation); err != nil {
		return err
	}

	return nil
}
