package getprovidersdatabyapplicationid

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
	applicationIDString := chi.URLParam(request, "applicationID")
	applicationIDUUID, err := uuid.Parse(applicationIDString)

	if err != nil {
		panic(err)
	}

	query := Query{
		ApplicationID: applicationIDUUID,
	}

	params := repositories.ParamsRs[Query, *Response, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: query,
	}

	response, errHandler := repositories.WithTransactionRs(request.Context(), params)

	if errHandler != nil {
		panic(errHandler)
	}

	http_router.SendJson(writter, response, http.StatusOK)
}
