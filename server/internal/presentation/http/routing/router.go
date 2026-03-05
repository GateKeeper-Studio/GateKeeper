package routing

import (
	"net/http"

	configureoauthprovider "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/configure-oauth-provider"
	getproviderdatabyid "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/get-provider-data-by-id"
	getprovidersdatabyapplicationid "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/get-providers-data-by-application-id"
	githubcallback "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/github-callback"
	githublogin "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/github-login"
	googlecallback "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/google-callback"
	googlelogin "github.com/gate-keeper/internal/features/handlers/application-oauth-provider/google-login"
	createrole "github.com/gate-keeper/internal/features/handlers/application-role/create-role"
	deleterole "github.com/gate-keeper/internal/features/handlers/application-role/delete-role"
	listroles "github.com/gate-keeper/internal/features/handlers/application-role/list-roles"
	createsecret "github.com/gate-keeper/internal/features/handlers/application-secret/create-secret"
	deletesecret "github.com/gate-keeper/internal/features/handlers/application-secret/delete-secret"
	createtenantuser "github.com/gate-keeper/internal/features/handlers/tenant-user/create-tenant-user"
	deletetenantuser "github.com/gate-keeper/internal/features/handlers/tenant-user/delete-tenant-user"
	edittenantuser "github.com/gate-keeper/internal/features/handlers/tenant-user/edit-tenant-user"
	gettenantuser "github.com/gate-keeper/internal/features/handlers/tenant-user/get-tenant-user-by-id"
	listtenantusers "github.com/gate-keeper/internal/features/handlers/tenant-user/list-tenant-users"
	listusersessions "github.com/gate-keeper/internal/features/handlers/tenant-user/list-user-sessions"
	revokeusersession "github.com/gate-keeper/internal/features/handlers/tenant-user/revoke-user-session"
	createapplication "github.com/gate-keeper/internal/features/handlers/application/create-application"
	getapplicationauthdata "github.com/gate-keeper/internal/features/handlers/application/get-application-auth-data"
	getapplicationbyid "github.com/gate-keeper/internal/features/handlers/application/get-application-by-id"
	listapplications "github.com/gate-keeper/internal/features/handlers/application/list-applications"
	removeapplication "github.com/gate-keeper/internal/features/handlers/application/remove-application"
	updateapplication "github.com/gate-keeper/internal/features/handlers/application/update-application"
	"github.com/gate-keeper/internal/features/handlers/authentication/authorize"
	beginwebauthnregistration "github.com/gate-keeper/internal/features/handlers/authentication/begin-webauthn-registration"
	changepassword "github.com/gate-keeper/internal/features/handlers/authentication/change-password"
	confirmmfaauthappsecret "github.com/gate-keeper/internal/features/handlers/authentication/confirm-mfa-auth-app-secret"
	confirmuseremail "github.com/gate-keeper/internal/features/handlers/authentication/confirm-user-email"
	forgotpassword "github.com/gate-keeper/internal/features/handlers/authentication/forgot-password"
	generateauthappsecret "github.com/gate-keeper/internal/features/handlers/authentication/generate-auth-app-secret"
	login "github.com/gate-keeper/internal/features/handlers/authentication/login"
	oidcdiscovery "github.com/gate-keeper/internal/features/handlers/authentication/oidc-discovery"
	resendemailconfirmation "github.com/gate-keeper/internal/features/handlers/authentication/resend-email-confirmation"
	resetpassword "github.com/gate-keeper/internal/features/handlers/authentication/reset-password"
	"github.com/gate-keeper/internal/features/handlers/authentication/session"
	signincredential "github.com/gate-keeper/internal/features/handlers/authentication/sign-in-credential"
	signupcredential "github.com/gate-keeper/internal/features/handlers/authentication/sign-up-credential"
	userinfo "github.com/gate-keeper/internal/features/handlers/authentication/userinfo"
	verifyappmfa "github.com/gate-keeper/internal/features/handlers/authentication/verify-app-mfa"
	verifyemailmfa "github.com/gate-keeper/internal/features/handlers/authentication/verify-email-mfa"
	verifywebauthnauth "github.com/gate-keeper/internal/features/handlers/authentication/verify-webauthn-authentication"
	verifywebauthnregistration "github.com/gate-keeper/internal/features/handlers/authentication/verify-webauthn-registration"

	accountchangepassword "github.com/gate-keeper/internal/features/handlers/account/change-password"
	accountconfirmemailchange "github.com/gate-keeper/internal/features/handlers/account/confirm-email-change"
	accountdisablemfamethod "github.com/gate-keeper/internal/features/handlers/account/disable-mfa-method"
	accountenableemailmfa "github.com/gate-keeper/internal/features/handlers/account/enable-email-mfa"
	accountgeneratebackupcodes "github.com/gate-keeper/internal/features/handlers/account/generate-backup-codes"
	accountgetlastmfatotpsecret "github.com/gate-keeper/internal/features/handlers/account/get-last-mfa-totp-secret-validation-by-user"
	accountlistmfamethods "github.com/gate-keeper/internal/features/handlers/account/list-mfa-methods"
	accountlistsessions "github.com/gate-keeper/internal/features/handlers/account/list-sessions"
	accountme "github.com/gate-keeper/internal/features/handlers/account/me"
	"github.com/gate-keeper/internal/features/handlers/account/reauthenticate"
	accountrefreshtoken "github.com/gate-keeper/internal/features/handlers/account/refresh-token"
	accountrequestemailchange "github.com/gate-keeper/internal/features/handlers/account/request-email-change"
	accountrevokeallsessions "github.com/gate-keeper/internal/features/handlers/account/revoke-all-sessions"
	accountrevokesession "github.com/gate-keeper/internal/features/handlers/account/revoke-session"
	accountupdatepreferredmfa "github.com/gate-keeper/internal/features/handlers/account/update-preferred-mfa"
	accountupdateprofile "github.com/gate-keeper/internal/features/handlers/account/update-profile"

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
	getProvidersDataByApplicationIDEndpoint := getprovidersdatabyapplicationid.Endpoint{DbPool: pool}

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

	createTenantUserEndpoint := createtenantuser.Endpoint{DbPool: pool}
	updateTenantUserEndpoint := edittenantuser.Endpoint{DbPool: pool}
	deleteTenantUserEndpoint := deletetenantuser.Endpoint{DbPool: pool}
	getTenantUserByIdEndpoint := gettenantuser.Endpoint{DbPool: pool}
	listTenantUsersEndpoint := listtenantusers.Endpoint{DbPool: pool}
	listUserSessionsEndpoint := listusersessions.Endpoint{DbPool: pool}
	revokeUserSessionEndpoint := revokeusersession.Endpoint{DbPool: pool}

	authorizeEndpoint := authorize.Endpoint{DbPool: pool}
	changePasswordEndpoint := changepassword.Endpoint{DbPool: pool}
	confirmUserEmailEndpoint := confirmuseremail.Endpoint{DbPool: pool}
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
	beginWebAuthnRegistrationEndpoint := beginwebauthnregistration.Endpoint{DbPool: pool}
	verifyWebAuthnRegistrationEndpoint := verifywebauthnregistration.Endpoint{DbPool: pool}
	verifyWebAuthnAuthEndpoint := verifywebauthnauth.Endpoint{DbPool: pool}

	githubLoginEndpoint := githublogin.Endpoint{DbPool: pool}
	githubCallbackEndpoint := githubcallback.Endpoint{DbPool: pool}

	googleLoginEndpoint := googlelogin.Endpoint{DbPool: pool}
	googleCallbackEndpoint := googlecallback.Endpoint{DbPool: pool}

	oidcDiscoveryEndpoint := oidcdiscovery.Endpoint{}
	userinfoEndpoint := userinfo.Endpoint{DbPool: pool}

	// Account (Self-Service Portal)
	reauthenticateEndpoint := reauthenticate.Endpoint{DbPool: pool}
	accountChangePasswordEndpoint := accountchangepassword.Endpoint{DbPool: pool}
	accountListMfaMethodsEndpoint := accountlistmfamethods.Endpoint{DbPool: pool}
	accountUpdatePreferredMfaEndpoint := accountupdatepreferredmfa.Endpoint{DbPool: pool}
	accountDisableMfaMethodEndpoint := accountdisablemfamethod.Endpoint{DbPool: pool}
	accountEnableEmailMfaEndpoint := accountenableemailmfa.Endpoint{DbPool: pool}
	accountGetLastMfaTotpSecretEndpoint := accountgetlastmfatotpsecret.Endpoint{DbPool: pool}
	accountListSessionsEndpoint := accountlistsessions.Endpoint{DbPool: pool}
	accountRevokeSessionEndpoint := accountrevokesession.Endpoint{DbPool: pool}
	accountRevokeAllSessionsEndpoint := accountrevokeallsessions.Endpoint{DbPool: pool}
	accountGenerateBackupCodesEndpoint := accountgeneratebackupcodes.Endpoint{DbPool: pool}
	accountRequestEmailChangeEndpoint := accountrequestemailchange.Endpoint{DbPool: pool}
	accountConfirmEmailChangeEndpoint := accountconfirmemailchange.Endpoint{DbPool: pool}
	accountMeEndpoint := accountme.Endpoint{DbPool: pool}
	accountUpdateProfileEndpoint := accountupdateprofile.Endpoint{DbPool: pool}
	accountRefreshTokenEndpoint := accountrefreshtoken.Endpoint{DbPool: pool}

	r := chi.NewRouter()

	r.Use(middleware.Logger)
	r.Use(middleware.RequestID)
	r.Use(middleware.Recoverer)

	r.Use(http_middlewares.ErrorHandler)

	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"*"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type", "X-CSRF-Token", "X-Step-Up-Token"},
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

	// OIDC Discovery document
	r.Get("/.well-known/openid-configuration", oidcDiscoveryEndpoint.Http)

	// Routes v1
	r.Route("/v1", func(r chi.Router) {
		r.Route("/auth", func(r chi.Router) {
			r.Route("/session", func(r chi.Router) {
				r.Use(http_middlewares.JwtHandler)
				r.Get("/", sessionEndpoint.Http)
			})

			r.Post("/authorize", authorizeEndpoint.Http)
			r.Post("/sign-in", signInCredentialEndpoint.Http)
			r.Get("/userinfo", userinfoEndpoint.Http)
			r.Post("/login", loginEndpoint.Http)
			r.Post("/generate-auth-secret", generateAuthAppSecretEndpoint.Http)
			r.Post("/verify-mfa/email", verfifyEmailMfaEndpoint.Http)
			r.Post("/verify-mfa/app", verfifyAppMfaEndpoint.Http)
			r.Post("/verify-mfa/webauthn", verifyWebAuthnAuthEndpoint.Http)
			r.Post("/sign-up", signUpCredentialEndpoint.Http)
			r.Post("/confirm-email", confirmUserEmailEndpoint.Http)
			r.Post("/reset-password", resetRepositoryEndpoint.Http)
			r.Post("/change-password", changePasswordEndpoint.Http)
			r.Post("/forgot-password", forgotPasswordEndpoint.Http)
			r.Post("/confirm-email/resend", resendEmailConfirmationEndpoint.Http)
			r.Post("/confirm-mfa-auth-app-secret", confirmMfaAuthAppSecretEndpoint.Http)

			r.Route("/webauthn", func(r chi.Router) {
				r.Post("/begin-registration", beginWebAuthnRegistrationEndpoint.Http)
				r.Post("/verify-registration", verifyWebAuthnRegistrationEndpoint.Http)
			})

			r.Get("/application/{applicationID}/auth-data", getApplicationAuthDataEndpoint.Http)

			r.Get("/application/oauth-provider/{applicationOAuthProviderID}", getProviderDataByIDEndpoint.Http)

			r.Route("/oauth-provider", func(r chi.Router) {
				r.Post("/github/login", githubLoginEndpoint.Http)
				r.Get("/github/callback", githubCallbackEndpoint.Http)

				r.Post("/google/login", googleLoginEndpoint.Http)
				r.Get("/google/callback", googleCallbackEndpoint.Http)
			})
		})

		r.Route("/account", func(r chi.Router) {
			r.Use(http_middlewares.JwtHandler)

			// Get current user profile
			r.Get("/me", accountMeEndpoint.Http)

			// Update profile
			r.Put("/profile", accountUpdateProfileEndpoint.Http)

			// Reauthenticate — issues a step-up token
			r.Post("/reauthenticate", reauthenticateEndpoint.Http)

			// Change password — requires step-up
			r.Route("/change-password", func(r chi.Router) {
				r.Use(http_middlewares.StepUpAuthHandler(pool))
				r.Post("/", accountChangePasswordEndpoint.Http)
			})

			// MFA management
			r.Route("/mfa", func(r chi.Router) {
				r.Get("/methods", accountListMfaMethodsEndpoint.Http)
				r.Put("/preferred", accountUpdatePreferredMfaEndpoint.Http)
				r.Post("/enable-email", accountEnableEmailMfaEndpoint.Http)
				r.Get("/totp-secret", accountGetLastMfaTotpSecretEndpoint.Http)
				r.Route("/methods/{method}", func(r chi.Router) {
					r.Use(http_middlewares.StepUpAuthHandler(pool))
					r.Delete("/", accountDisableMfaMethodEndpoint.Http)
				})
			})

			// Session management
			r.Route("/sessions", func(r chi.Router) {
				r.Group(func(r chi.Router) {
					r.Use(http_middlewares.StepUpAuthHandler(pool))
					r.Get("/", accountListSessionsEndpoint.Http)
					r.Delete("/", accountRevokeAllSessionsEndpoint.Http)
				})
				r.Delete("/{sessionID}", accountRevokeSessionEndpoint.Http)
			})

			// Backup codes — requires step-up
			r.Route("/backup-codes", func(r chi.Router) {
				r.Use(http_middlewares.StepUpAuthHandler(pool))
				r.Post("/", accountGenerateBackupCodesEndpoint.Http)
			})

			// Email change
			r.Route("/email", func(r chi.Router) {
				r.Route("/change", func(r chi.Router) {
					r.Use(http_middlewares.StepUpAuthHandler(pool))
					r.Post("/", accountRequestEmailChangeEndpoint.Http)
				})
			})
		})

		// Refresh token — uses JwtRefreshHandler (30-min leeway for expired tokens)
		r.Route("/account/refresh", func(r chi.Router) {
			r.Use(http_middlewares.JwtRefreshHandler)
			r.Post("/", accountRefreshTokenEndpoint.Http)
		})

		// Confirm email change — public (token-based, no JWT required)
		r.Post("/account/email/confirm", accountConfirmEmailChangeEndpoint.Http)

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
						r.Get("/", listTenantUsersEndpoint.Http)
						r.Post("/", createTenantUserEndpoint.Http)
						r.Put("/{userID}", updateTenantUserEndpoint.Http)
						r.Get("/{userID}", getTenantUserByIdEndpoint.Http)
						r.Delete("/{userID}", deleteTenantUserEndpoint.Http)

						r.Get("/{userID}/sessions", listUserSessionsEndpoint.Http)
						r.Delete("/{userID}/sessions/{sessionID}", revokeUserSessionEndpoint.Http)
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
						r.Get("/", getProvidersDataByApplicationIDEndpoint.Http)
						r.Put("/", configureOauthProviderEndPoint.Http)
					})
				})
			})
		})
	})

	return r
}
