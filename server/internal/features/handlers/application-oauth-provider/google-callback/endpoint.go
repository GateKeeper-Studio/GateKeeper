package googlecallback

import (
	"net/http"
	"net/url"
	"os"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

// getMfaPagePath maps the MFA type from the policy engine to the IDP auth page path.
func getMfaPagePath(mfaType string) string {
	switch mfaType {
	case "email":
		return "mfa-mail"
	case "totp":
		return "mfa-app"
	case "webauthn":
		return "mfa-webauthn"
	default:
		return "mfa-app"
	}
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	parsedUrl, err := url.Parse(request.RequestURI)

	if err != nil {
		http_router.SendJson(writter, "Error on trying to parse the URL", http.StatusBadRequest)
		return
	}

	code := parsedUrl.Query().Get("code")
	state := parsedUrl.Query().Get("state")

	command := Command{
		Code:  code,
		State: state,
	}

	params := repositories.ParamsRs[Command, *ServiceResponse, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	response, errHandler := repositories.WithTransactionRs(request.Context(), params)

	if errHandler != nil {
		panic(errHandler)
	}

	if response.MfaRequired && response.MfaChallenge != nil {
		// MFA required — redirect to the Identity Provider's MFA page
		mfaPath := getMfaPagePath(response.MfaChallenge.MfaType)
		dashboardURL := os.Getenv("DASHBOARD_URL")
		idpMfaURL := dashboardURL + "/auth/" + response.MfaChallenge.ApplicationID.String() + "/" + mfaPath

		mfaRedirect, err := url.Parse(idpMfaURL)
		if err != nil {
			http_router.SendJson(writter, "Error on trying to parse the IDP MFA URL", http.StatusBadRequest)
			return
		}

		query := mfaRedirect.Query()
		query.Set("redirect_uri", response.ClientRedirectUri)
		query.Set("email", response.MfaChallenge.Email)
		query.Set("mfa_id", response.MfaChallenge.MfaID.String())
		query.Set("state", response.ClientState)
		query.Set("code_challenge", response.ClientCodeChallenge)
		query.Set("code_challenge_method", response.ClientCodeChallengeMethod)
		query.Set("scope", response.ClientScope)
		query.Set("response_type", response.ClientResponseType)
		if response.ClientNonce != nil {
			query.Set("nonce", *response.ClientNonce)
		}
		if response.MfaChallenge.WebAuthnOptions != nil {
			query.Set("webauthn_options", string(*response.MfaChallenge.WebAuthnOptions))
		}

		mfaRedirect.RawQuery = query.Encode()
		http.Redirect(writter, request, mfaRedirect.String(), http.StatusFound)
		return
	}

	// No MFA required — redirect to client callback with authorization code
	redirectUrl, err := url.Parse(response.RedirectURL)

	if err != nil {
		http_router.SendJson(writter, "Error on trying to parse the Redirect URL", http.StatusBadRequest)
		return
	}

	query := redirectUrl.Query()
	query.Set("code", response.AuthorizationCode)
	query.Set("state", response.ClientState)
	query.Set("code_challenge_method", response.ClientCodeChallengeMethod)
	query.Set("code_challenge", response.ClientCodeChallenge)
	query.Set("redirect_uri", response.ClientRedirectUri)
	query.Set("scope", response.ClientScope)
	query.Set("response_type", response.ClientResponseType)

	redirectUrl.RawQuery = query.Encode()

	http.Redirect(writter, request, redirectUrl.String(), http.StatusFound)
}
