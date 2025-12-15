package githubcallback

import (
	"net/http"
	"net/url"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	// oauthState, err := request.Cookie("oauth_state")

	// log.Println("Test0: " + err.Error())

	// if err != nil {
	// 	http_router.SendJson(writter, nil, http.StatusBadRequest)
	// 	return
	// }

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

	redirectUrl, err := url.Parse(response.RedirectURL)

	if err != nil {
		http_router.SendJson(writter, "Error on trying to parse the Redirect URL", http.StatusBadRequest)
		return
	}

	query := redirectUrl.Query()
	query.Set("code", "") // to do
	query.Set("state", response.ClientState)
	query.Set("code_challenge_method", response.ClientCodeChallengeMethod)
	query.Set("code_challenge", response.ClientCodeChallenge)
	query.Set("redirect_uri", response.ClientRedirectUri)
	query.Set("scope", response.ClientScope)
	query.Set("response_type", response.ClientResponseType)

	redirectUrl.RawQuery = query.Encode()

	http.Redirect(writter, request, redirectUrl.String(), http.StatusFound)
	// http_router.SendJson(writter, nil, http.StatusOK)
}
