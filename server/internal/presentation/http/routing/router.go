package routing

import (
	"net/http"

	configureoauthprovider "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/configure-oauth-provider"
	getproviderdatabyid "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/get-provider-data-by-id"
	githubcallback "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/github-callback"
	githublogin "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/github-login"
	createrole "github.com/gate-keeper/internal/features/handlers/application-role/create-role"
	deleterole "github.com/gate-keeper/internal/features/handlers/application-role/delete-role"
	listroles "github.com/gate-keeper/internal/features/handlers/application-role/list-roles"
	createsecret "github.com/gate-keeper/internal/features/handlers/application-secret/create-secret"
	deletesecret "github.com/gate-keeper/internal/features/handlers/application-secret/delete-secret"
	createapplicationuser "github.com/gate-keeper/internal/features/handlers/application-user/create-application-user"
	deleteapplicationuser "github.com/gate-keeper/internal/features/handlers/application-user/delete-application-user"
	editapplicationuser "github.com/gate-keeper/internal/features/handlers/application-user/edit-application-user"
	getapplicationuserbyid "github.com/gate-keeper/internal/features/handlers/application-user/get-application-user-by-id"
	createapplication "github.com/gate-keeper/internal/features/handlers/application/create-application"
	getapplicationauthdata "github.com/gate-keeper/internal/features/handlers/application/get-application-auth-data"
	getapplicationbyid "github.com/gate-keeper/internal/features/handlers/application/get-application-by-id"
	listapplications "github.com/gate-keeper/internal/features/handlers/application/list-applications"
	removeapplication "github.com/gate-keeper/internal/features/handlers/application/remove-application"
	updateapplication "github.com/gate-keeper/internal/features/handlers/application/update-application"
	"github.com/gate-keeper/internal/features/handlers/authentication/authorize"
	changepassword "github.com/gate-keeper/internal/features/handlers/authentication/change-password"
	confirmmfaauthappsecret "github.com/gate-keeper/internal/features/handlers/authentication/confirm-mfa-auth-app-secret"
	confirmuseremail "github.com/gate-keeper/internal/features/handlers/authentication/confirm-user-email"
	externalloginprovider "github.com/gate-keeper/internal/features/handlers/authentication/external-login-provider"
	forgotpassword "github.com/gate-keeper/internal/features/handlers/authentication/forgot-password"
	generateauthappsecret "github.com/gate-keeper/internal/features/handlers/authentication/generate-auth-app-secret"
	login "github.com/gate-keeper/internal/features/handlers/authentication/login"
	resendemailconfirmation "github.com/gate-keeper/internal/features/handlers/authentication/resend-email-confirmation"
	resetpassword "github.com/gate-keeper/internal/features/handlers/authentication/reset-password"
	"github.com/gate-keeper/internal/features/handlers/authentication/session"
	signincredential "github.com/gate-keeper/internal/features/handlers/authentication/sign-in-credential"
	signupcredential "github.com/gate-keeper/internal/features/handlers/authentication/sign-up-credential"
	verifyappmfa "github.com/gate-keeper/internal/features/handlers/authentication/verify-app-mfa"
	verifyemailmfa "github.com/gate-keeper/internal/features/handlers/authentication/verify-email-mfa"
	createorganization "github.com/gate-keeper/internal/features/handlers/organization/create-organization"
	editorganization "github.com/gate-keeper/internal/features/handlers/organization/edit-organization"
	getorganizationbyid "github.com/gate-keeper/internal/features/handlers/organization/get-organization-by-id"
	listorganizations "github.com/gate-keeper/internal/features/handlers/organization/list-organizations"
	removeorganization "github.com/gate-keeper/internal/features/handlers/organization/remove-organization"
	http_middlewares "github.com/gate-keeper/internal/presentation/http/middlewares"

	"github.com/go-chi/chi"
	"github.com/go-chi/chi/middleware"
	"github.com/go-chi/cors"
	"github.com/jackc/pgx/v5/pgxpool"
	httpSwagger "github.com/swaggo/http-swagger"
)

func SetHttpRoutes(pool *pgxpool.Pool) http.Handler {
	listApplicationsEndpoint := listapplications.Endpoint{DbPool: pool}
	updateApplicationEndpoint := updateapplication.Endpoint{DbPool: pool}
	removeApplicationEndpoint := removeapplication.Endpoint{DbPool: pool}
	createApplicationEndpoint := createapplication.Endpoint{DbPool: pool}
	getApplicationByIdEndpoint := getapplicationbyid.Endpoint{DbPool: pool}
	getApplicationAuthDataEndpoint := getapplicationauthdata.Endpoint{DbPool: pool}

	configureOauthProviderEndPoint := configureoauthprovider.Endpoint{DbPool: pool}
	getProviderDataByIDEndpoint := getproviderdatabyid.Endpoint{DbPool: pool}

	listRolesEndpoint := listroles.Endpoint{DbPool: pool}
	createRoleEndpoint := createrole.Endpoint{DbPool: pool}
	deleteRoleEndpoint := deleterole.Endpoint{DbPool: pool}

	createEndpoint := createsecret.Endpoint{DbPool: pool}
	deleteSecretEndpoint := deletesecret.Endpoint{DbPool: pool}

	getOrganizationByIdEndpoint := getorganizationbyid.Endpoint{DbPool: pool}
	createOrganizationEndpoint := createorganization.Endpoint{DbPool: pool}
	listOrganizationsEndpoint := listorganizations.Endpoint{DbPool: pool}
	removeOrganizationEndpoint := removeorganization.Endpoint{DbPool: pool}
	editOrganizationEndpoint := editorganization.Endpoint{DbPool: pool}

	createApplicationUserEndpoint := createapplicationuser.Endpoint{DbPool: pool}
	updateApplicationUserEndpoint := editapplicationuser.Endpoint{DbPool: pool}
	deleteApplicationUserEndpoint := deleteapplicationuser.Endpoint{DbPool: pool}
	getApplicationUserByIdEndpoint := getapplicationuserbyid.Endpoint{DbPool: pool}

	authorizeEndpoint := authorize.Endpoint{DbPool: pool}
	changePasswordEndpoint := changepassword.Endpoint{DbPool: pool}
	confirmUserEmailEndpoint := confirmuseremail.Endpoint{DbPool: pool}
	externalLoginProviderEndpoint := externalloginprovider.Endpoint{DbPool: pool}
	sessionEndpoint := session.Endpoint{DbPool: pool}
	forgotPasswordEndpoint := forgotpassword.Endpoint{DbPool: pool}
	generateAuthAppSecretEndpoint := generateauthappsecret.Endpoint{DbPool: pool}
	loginEndpoint := login.Endpoint{DbPool: pool}
	resendEmailConfirmationEndpoint := resendemailconfirmation.Endpoint{DbPool: pool}
	resetRepositoryEndpoint := resetpassword.Endpoint{DbPool: pool}
	signInCredentialEndpoint := signincredential.Endpoint{DbPool: pool}
	signUpCredentialEndpoint := signupcredential.Endpoint{DbPool: pool}
	verfifyEmailMfaEndpoint := verifyemailmfa.Endpoint{DbPool: pool}
	verfifyAppMfaEndpoint := verifyappmfa.Endpoint{DbPool: pool}
	confirmMfaAuthAppSecretEndpoint := confirmmfaauthappsecret.Endpoint{DbPool: pool}

	githubLoginEndpoint := githublogin.Endpoint{DbPool: pool}
	githubCallbackEndpoint := githubcallback.Endpoint{DbPool: pool}

	r := chi.NewRouter()

	r.Use(middleware.Logger)
	r.Use(middleware.RequestID)
	r.Use(middleware.Recoverer)

	r.Use(http_middlewares.ErrorHandler)

	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"*"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type", "X-CSRF-Token"},
		ExposedHeaders:   []string{"Link"},
		AllowCredentials: false,
		MaxAge:           300, // 5 minutes
	}))

	r.Get("/swagger/*", httpSwagger.Handler(
		httpSwagger.URL("http://localhost:8080/swagger/doc.json"), //The url pointing to API definition
	))

	// Health check
	r.Get("/health", func(writter http.ResponseWriter, request *http.Request) {
		writter.Write([]byte("Healthy"))
	})

	// Routes v1
	r.Route("/v1", func(r chi.Router) {
		r.Route("/auth", func(r chi.Router) {
			r.Route("/session", func(r chi.Router) {
				r.Use(http_middlewares.JwtHandler)
				r.Get("/", sessionEndpoint.Http)
			})

			r.Post("/authorize", authorizeEndpoint.Http)
			r.Post("/sign-in", signInCredentialEndpoint.Http)
			r.Post("/login", loginEndpoint.Http)
			r.Post("/generate-auth-secret", generateAuthAppSecretEndpoint.Http)
			r.Post("/verify-mfa/email", verfifyEmailMfaEndpoint.Http)
			r.Post("/verify-mfa/app", verfifyAppMfaEndpoint.Http)
			r.Post("/sign-up", signUpCredentialEndpoint.Http)
			r.Post("/confirm-email", confirmUserEmailEndpoint.Http)
			r.Post("/reset-password", resetRepositoryEndpoint.Http)
			r.Post("/change-password", changePasswordEndpoint.Http)
			r.Post("/forgot-password", forgotPasswordEndpoint.Http)
			r.Post("/external-provider", externalLoginProviderEndpoint.Http)
			r.Post("/confirm-email/resend", resendEmailConfirmationEndpoint.Http)
			r.Post("/confirm-mfa-auth-app-secret", confirmMfaAuthAppSecretEndpoint.Http)

			r.Get("/application/{applicationID}/auth-data", getApplicationAuthDataEndpoint.Http)

			r.Get("/application/oauth-provider/{applicationOAuthProviderID}", getProviderDataByIDEndpoint.Http)

			r.Route("/oauth-provider", func(r chi.Router) {
				r.Post("/github/login", githubLoginEndpoint.Http)
				r.Get("/github/callback", githubCallbackEndpoint.Http)
			})
		})

		r.Route("/organizations", func(r chi.Router) {
			// r.Use(http_middlewares.JwtHandler)

			r.Get("/", listOrganizationsEndpoint.Http)
			r.Post("/", createOrganizationEndpoint.Http)

			r.Route("/{organizationID}", func(r chi.Router) {
				r.Get("/", getOrganizationByIdEndpoint.Http)
				r.Delete("/", removeOrganizationEndpoint.Http)
				r.Put("/", editOrganizationEndpoint.Http)

				r.Route("/applications", func(r chi.Router) {
					r.Get("/", listApplicationsEndpoint.Http)
					r.Post("/", createApplicationEndpoint.Http)
					r.Put("/{applicationID}", updateApplicationEndpoint.Http)
					r.Get("/{applicationID}", getApplicationByIdEndpoint.Http)
					r.Delete("/{applicationID}", removeApplicationEndpoint.Http)

					r.Route("/{applicationID}/users", func(r chi.Router) {
						r.Post("/", createApplicationUserEndpoint.Http)
						r.Put("/{userID}", updateApplicationUserEndpoint.Http)
						r.Get("/{userID}", getApplicationUserByIdEndpoint.Http)
						r.Delete("/{userID}", deleteApplicationUserEndpoint.Http)
					})

					r.Route("/{applicationID}/roles", func(r chi.Router) {
						r.Get("/", listRolesEndpoint.Http)
						r.Post("/", createRoleEndpoint.Http)
						r.Delete("/{roleID}", deleteRoleEndpoint.Http)
					})

					r.Route("/{applicationID}/secrets", func(r chi.Router) {
						r.Post("/", createEndpoint.Http)
						r.Delete("/{secretID}", deleteSecretEndpoint.Http)
					})

					r.Route("/{applicationID}/oauth-provider", func(r chi.Router) {
						r.Put("/", configureOauthProviderEndPoint.Http)
					})
				})
			})
		})
	})

	return r
}
