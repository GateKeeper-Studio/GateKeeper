package getproviderdatabyid

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
	oauthProviderIDString := chi.URLParam(request, "applicationOAuthProviderID")
	oauthProviderIDUUID, err := uuid.Parse(oauthProviderIDString)

	if err != nil {
		panic(err)
	}

	query := Query{
		ApplicaionOAuthProviderID: oauthProviderIDUUID,
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
