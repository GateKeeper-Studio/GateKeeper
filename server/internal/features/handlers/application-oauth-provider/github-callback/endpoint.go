package githubcallback

import (
	"net/http"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	oauthState, err := request.Cookie("oauth_state")
	if err != nil {
		http_router.SendJson(writter, nil, http.StatusBadRequest)
		return
	}

	command := Command{
		
		StoredState: oauthState.Value,
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

	http.SetCookie(writter, &http.Cookie{
		Name:   "oauth_state",
		Value:  "",
		Path:   "/",
		MaxAge: -1,
	})

	http.SetCookie(writter, &http.Cookie{
		Name:   "oauth_provider_id",
		Value:  "",
		Path:   "/",
		MaxAge: -1,
	})

	http.Redirect(writter, request, response.RedirectURL, http.StatusFound)
	// http_router.SendJson(writter, nil, http.StatusOK)
}
