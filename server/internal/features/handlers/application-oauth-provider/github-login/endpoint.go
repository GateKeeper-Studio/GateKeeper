package githublogin

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
	queryParams.Set("application_id", serviceResponse.ApplicationID.String())

	queryString := queryParams.Encode()

	githubUrl, _ := url.Parse("https://github.com/login/oauth/authorize")
	githubUrl.RawQuery = queryString // Set the query parameters for the GitHub URL

	response := Response{
		Url:   githubUrl.String(),
		State: serviceResponse.State,
	}

	http_router.SendJson(writter, response, http.StatusOK)
}
