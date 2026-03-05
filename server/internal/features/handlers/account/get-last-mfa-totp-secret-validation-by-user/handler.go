package accountgetlastmfatotpsecret

import (
	"context"
	"fmt"
	"net/url"

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

func (h *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// Clean up expired secrets
	if err := h.repository.DeleteExpiredMfaTotpSecretValidationByUserID(ctx, command.UserID); err != nil {
		return nil, err
	}

	// Get the last valid (non-expired, non-validated) secret
	secret, err := h.repository.GetLastValidMfaTotpSecretByUserID(ctx, command.UserID)
	if err != nil {
		return nil, &errors.ErrMfaUserSecretNotFound
	}
	if secret == nil {
		return nil, &errors.ErrMfaUserSecretNotFound
	}

	// Reconstruct the OTP URL using the tenant name as the issuer
	tenant, err := h.repository.GetTenantByID(ctx, user.TenantID)
	if err != nil {
		return nil, err
	}
	if tenant == nil {
		return nil, &errors.ErrTenantNotFound
	}

	otpUrl := fmt.Sprintf(
		"otpauth://totp/%s:%s?secret=%s&issuer=%s&algorithm=SHA1&digits=6&period=30",
		url.PathEscape(tenant.Name),
		url.PathEscape(user.Email),
		secret.Secret,
		url.QueryEscape(tenant.Name),
	)

	return &Response{
		OtpUrl:    otpUrl,
		ExpiresAt: secret.ExpiresAt,
	}, nil
}
