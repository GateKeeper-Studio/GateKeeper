package accountrevokesession

import (
	"net/http"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	sessionIDStr := chi.URLParam(request, "sessionID")
	sessionID, err := uuid.Parse(sessionIDStr)
	if err != nil {
		panic(err)
	}

	command := Command{
		UserID:    http_router.GetUserIDFromContext(request.Context()),
		SessionID: sessionID,
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
