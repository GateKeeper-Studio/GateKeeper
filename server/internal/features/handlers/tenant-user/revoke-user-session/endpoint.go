package revokeusersession

import (
	"net/http"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/go-chi/chi"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	userIDString := chi.URLParam(request, "userID")
	userIdUUID, err := uuid.Parse(userIDString)

	if err != nil {
		panic(err)
	}

	sessionIDString := chi.URLParam(request, "sessionID")
	sessionIdUUID, err := uuid.Parse(sessionIDString)

	if err != nil {
		panic(err)
	}

	command := Command{
		UserID:    userIdUUID,
		SessionID: sessionIdUUID,
	}

	params := repositories.Params[Command, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	if err := repositories.WithTransaction(request.Context(), params); err != nil {
		panic(err)
	}

	http_router.SendJson(writter, nil, http.StatusNoContent)
}
