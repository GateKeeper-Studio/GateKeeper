package googlelogin

import (
	"net/http"
	"net/url"
	"time"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	var command Command

	if err := http_router.ParseBodyToSchema(&command, request); err != nil {
		panic(err)
	}

	params := repositories.ParamsRs[Command, *ServiceResponse, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	serviceResponse, errHandler := repositories.WithTransactionRs(request.Context(), params)

	if errHandler != nil {
		panic(errHandler)
	}

	timestamp := time.Minute * 10

	http.SetCookie(writter, &http.Cookie{
		Name:     "oauth_state",
		Value:    serviceResponse.State,
		HttpOnly: true,
		Secure:   true,
		Path:     "/",
		MaxAge:   int(timestamp.Seconds()), // 10 minutes
	})

	http.SetCookie(writter, &http.Cookie{
		Name:     "oauth_provider_id",
		Value:    command.OauthProviderId.String(),
		HttpOnly: true,
		Secure:   true,
		Path:     "/",
		MaxAge:   int(timestamp.Seconds()), // 10 minutes
	})

	queryParams := request.URL.Query()

	queryParams.Set("client_id", serviceResponse.ClientID)
	queryParams.Set("redirect_uri", serviceResponse.RedirectURI)
	queryParams.Set("scope", serviceResponse.Scope)
	queryParams.Set("state", serviceResponse.State)
	queryParams.Set("response_type", "code")
	queryParams.Set("code_challenge", serviceResponse.CodeChallenge)
	queryParams.Set("code_challenge_method", serviceResponse.CodeChallengeMethod)

	queryString := queryParams.Encode()

	googleUrl, _ := url.Parse("https://accounts.google.com/o/oauth2/v2/auth")
	googleUrl.RawQuery = queryString // Set the query parameters for the Google URL

	response := Response{
		Url:   googleUrl.String(),
		State: serviceResponse.State,
	}

	http_router.SendJson(writter, response, http.StatusOK)
}
