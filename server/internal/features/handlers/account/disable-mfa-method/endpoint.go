package accountdisablemfamethod

import (
	"net/http"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/go-chi/chi"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	method := chi.URLParam(request, "method")

	command := Command{
		UserID:    http_router.GetUserIDFromContext(request.Context()),
		Method:    method,
		IPAddress: request.RemoteAddr,
		UserAgent: request.UserAgent(),
	}

	params := repositories.ParamsRs[Command, *Response, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	response, err := repositories.WithTransactionRs(request.Context(), params)
	if err != nil {
		panic(err)
	}

	http_router.SendJson(writter, response, http.StatusOK)
}
