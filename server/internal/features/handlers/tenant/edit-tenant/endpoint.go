package edittenant

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
	tenantIDString := chi.URLParam(request, "tenantID")
	tenantIdUUID, err := uuid.Parse(tenantIDString)

	if err != nil {
		panic(err)
	}

	var requestBody RequestBody

	if err := http_router.ParseBodyToSchema(&requestBody, request); err != nil {
		panic(err)
	}

	command := Command{
		ID:          tenantIdUUID,
		Name:        requestBody.Name,
		Description: requestBody.Description,
	}

	params := repositories.Params[Command, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	errHandler := repositories.WithTransaction(request.Context(), params)

	if errHandler != nil {
		panic(errHandler)
	}

	http_router.SendJson(writter, nil, http.StatusNoContent)
}
