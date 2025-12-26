package errors

import (
	"net/http"
)

type CustomError struct {
	Name    string
	Code    int
	Message string
	Title   string
}

func (e *CustomError) Error() string {
	return e.Name
}

var (
	ErrUserNotFound           = CustomError{Name: "ErrUserNotFound", Code: http.StatusNotFound, Message: "User was not found in the system", Title: "User not found"}
	ErrEmailOrPasswordInvalid = CustomError{Name: "ErrEmailOrPasswordInvalid", Code: http.StatusBadRequest, Message: "E-mail/password is incorrect or invalid", Title: "Invalid e-mail or password"}
	ErrInvalidEmail           = CustomError{Name: "ErrInvalidEmail", Code: http.StatusBadRequest, Message: "Invalid e-mail address, please provide a valid e-mail address", Title: "Invalid e-mail"}
	ErrEmailNotConfirmed      = CustomError{Name: "ErrEmailNotConfirmed", Code: http.StatusBadRequest, Message: "E-mail not confirmed, please confirm your e-mail address to continue", Title: "E-mail not confirmed"}
	ErrUserNotActive          = CustomError{Name: "ErrUserNotActive", Code: http.StatusBadRequest, Message: "User not active, please contact support", Title: "User not active"}
	ErrUserAlreadyExists      = CustomError{Name: "ErrUserAlreadyExists", Code: http.StatusBadRequest, Message: "An user is already registered with this e-mail, try another e-mail", Title: "User already exists"}
	ErrUserSignUpWithSocial   = CustomError{Name: "ErrUserSignUpWithSocial", Code: http.StatusBadRequest, Message: "User signed up with social login, please use social login", Title: "User signed up with social login"}
	ErrMfaAuthAppNotEnabled   = CustomError{Name: "ErrMfaAuthAppNotEnabled", Code: http.StatusBadRequest, Message: "MFA Auth App not enabled to user", Title: "MFA Auth App not enabled"}

	ErrEmailConfirmationIsInCoolDown   = CustomError{Name: "ErrEmailConfirmationIsInCoolDown", Code: http.StatusBadRequest, Message: "E-mail confirmation is in cool down yet, wait a few minutes and try again", Title: "E-mail confirmation is in cool down"}
	ErrEmailConfirmationNotFound       = CustomError{Name: "ErrEmailConfirmationNotFound", Code: http.StatusBadRequest, Message: "E-mail confirmation not found", Title: "E_mail confirmation not found"}
	ErrConfirmationTokenAlreadyExpired = CustomError{Name: "ErrConfirmationTokenAlreadyExpired", Code: http.StatusBadRequest, Message: "Confirmation token already expired, try generating another one", Title: "Confirmation token already expired"}
	ErrConfirmationTokenAlreadyUsed    = CustomError{Name: "ErrConfirmationTokenAlreadyUsed", Code: http.StatusBadRequest, Message: "Confirmation token already used", Title: "Confirmation token already used"}
	ErrConfirmationTokenInvalid        = CustomError{Name: "ErrConfirmationTokenInvalid", Code: http.StatusBadRequest, Message: "Confirmation token invalid", Title: "Confirmation token invalid"}

	ErrPasswordResetNotFound      = CustomError{Name: "ErrPasswordResetNotFound", Code: http.StatusNotFound, Message: "Password reset token not found", Title: "Password reset token not found"}
	ErrPasswordResetTokenMismatch = CustomError{Name: "ErrPasswordResetTokenMismatch", Code: http.StatusBadRequest, Message: "Password reset token mismatch", Title: "Password reset token mismatch"}
	ErrPasswordResetTokenExpired  = CustomError{Name: "ErrPasswordResetTokenExpired", Code: http.StatusBadRequest, Message: "Password reset token expired", Title: "Password reset token expired"}

	ErrInvalidHash         = CustomError{Name: "ErrInvalidHash", Code: http.StatusBadRequest, Message: "The encoded hash is invalid", Title: "Invalid hash"}
	ErrIncompatibleVersion = CustomError{Name: "ErrIncompatibleVersion", Code: http.StatusBadRequest, Message: "Incompatible version of the hash algorithm", Title: "Incompatible version"}

	ErrApplicationNotFound      = CustomError{Name: "ErrApplicationNotFound", Code: http.StatusNotFound, Message: "Application not found", Title: "Application not found"}
	ErrAplicationSecretNotFound = CustomError{Name: "ErrAplicationSecretNotFound", Code: http.StatusNotFound, Message: "Application secret not found", Title: "Application secret not found"}
	ErrInvalidClientSecret      = CustomError{Name: "ErrInvalidClientSecret", Code: http.StatusBadRequest, Message: "Invalid client secret", Title: "Invalid client secret"}
	ErrClientSecretExpired      = CustomError{Name: "ErrClientSecretExpired", Code: http.StatusBadRequest, Message: "Client secret expired", Title: "Client secret expired"}

	ErrOrganizationNotFound = CustomError{Name: "ErrOrganizationNotFound", Code: http.StatusNotFound, Message: "Organization not found", Title: "Organization not found"}

	ErrUserRoleNotFound = CustomError{Name: "ErrUserRoleNotFound", Code: http.StatusNotFound, Message: "User role not found", Title: "User role not found"}

	ErrAuthorizationCodeNotFound           = CustomError{Name: "ErrAuthorizationCodeNotFound", Code: http.StatusNotFound, Message: "Authorization code not found", Title: "Authorization code not found"}
	ErrAuthorizationCodeExpired            = CustomError{Name: "ErrAuthorizationCodeExpired", Code: http.StatusBadRequest, Message: "Authorization code expired", Title: "Authorization code expired"}
	ErrAuthorizationCodeInvalidRedirectURI = CustomError{Name: "ErrAuthorizationCodeInvalidRedirectURI", Code: http.StatusBadRequest, Message: "Invalid redirect URI", Title: "Invalid redirect URI"}
	ErrAuthorizationCodeInvalidClientID    = CustomError{Name: "ErrAuthorizationCodeInvalidClientID", Code: http.StatusBadRequest, Message: "Invalid client ID", Title: "Invalid client ID"}
	ErrAuthorizationCodeInvalidPKCE        = CustomError{Name: "ErrAuthorizationCodeInvalidPKCE", Code: http.StatusBadRequest, Message: "Invalid PKCE", Title: "Invalid PKCE"}

	ErrSessionCodeNotFound    = CustomError{Name: "ErrSessionCodeNotFound", Code: http.StatusNotFound, Message: "Session code not found", Title: "Session code not found"}
	ErrSessionCodeExpired     = CustomError{Name: "ErrSessionCodeExpired", Code: http.StatusBadRequest, Message: "Session code expired", Title: "Session code expired"}
	ErrSessionCodeAlreadyUsed = CustomError{Name: "ErrSessionCodeAlreadyUsed", Code: http.StatusBadRequest, Message: "Session code already used", Title: "Session code already used"}

	ErrEmailMfaCodeExpired  = CustomError{Name: "ErrEmailMfaCodeExpired", Code: http.StatusBadRequest, Message: "E-mail MFA code expired", Title: "E-mail MFA code expired"}
	ErrEmailMfaCodeNotFound = CustomError{Name: "ErrEmailMfaCodeNotFound", Code: http.StatusNotFound, Message: "E-mail MFA code invalid", Title: "E-mail MFA code not found"}
	ErrMfaEmailNotEnabled   = CustomError{Name: "ErrMfaEmailNotEnabled", Code: http.StatusBadRequest, Message: "MFA e-mail not enabled", Title: "MFA e-mail not enabled to user"}

	ErrAppMfaCodeNotFound = CustomError{Name: "ErrAppMfaCodeNotFound", Code: http.StatusNotFound, Message: "MFA app code not found", Title: "MFA app code not found"}
	ErrAppMfaCodeExpired  = CustomError{Name: "ErrAppMfaCodeExpired", Code: http.StatusBadRequest, Message: "MFA app code expired", Title: "MFA app code expired"}
	ErrMfaAppNotEnabled   = CustomError{Name: "ErrMfaAppNotEnabled", Code: http.StatusBadRequest, Message: "MFA app not enabled to user", Title: "MFA app not enabled to user"}

	ErrChangePasswordCodeNotFound  = CustomError{Name: "ErrChangePasswordCodeNotFound", Code: http.StatusNotFound, Message: "Change password code not found", Title: "Change password code not found"}
	ErrChangePasswordCodeExpired   = CustomError{Name: "ErrChangePasswordCodeExpired", Code: http.StatusBadRequest, Message: "Change password code expired", Title: "Change password code expired"}
	ErrChangePasswordTokenMismatch = CustomError{Name: "ErrChangePasswordTokenMismatch", Code: http.StatusBadRequest, Message: "Change password token mismatch", Title: "Change password token mismatch"}
	ErrUserShouldNotChangePassword = CustomError{Name: "ErrUserShouldNotChangePassword", Code: http.StatusBadRequest, Message: "User should not change password", Title: "User should not change password"}
	ErrUserShouldChangePassword    = CustomError{Name: "ErrUserShouldChangePassword", Code: http.StatusBadRequest, Message: "User should change password", Title: "User should change password"}

	ErrMfaUserSecretNotFound         = CustomError{Name: "ErrMfaUserSecretNotFound", Code: http.StatusNotFound, Message: "MFA user secret not found", Title: "MFA user secret not found"}
	ErrInvalidMfaAuthAppCode         = CustomError{Name: "ErrInvalidMfaAuthAppCode", Code: http.StatusBadRequest, Message: "Invalid MFA Auth App code", Title: "Invalid MFA Auth App code"}
	ErrMfaUserSecretAlreadyValidated = CustomError{Name: "ErrMfaUserSecretAlreadyValidated", Code: http.StatusBadRequest, Message: "MFA user secret already validated", Title: "MFA user secret already validated"}

	ErrMfaCodeNotFound = CustomError{Name: "ErrMfaCodeNotFound", Code: http.StatusNotFound, Message: "MFA code not found", Title: "MFA code not found"}
	ErrMfaCodeExpired  = CustomError{Name: "ErrMfaCodeExpired", Code: http.StatusBadRequest, Message: "MFA code expired", Title: "MFA code expired"}
	ErrMfaCodeRequired = CustomError{Name: "ErrMfaCodeRequired", Code: http.StatusBadRequest, Message: "MFA code is required", Title: "MFA code required"}

	ErrInvalid2FAMethod    = CustomError{Name: "ErrInvalid2FAMethod", Code: http.StatusBadRequest, Message: "Invalid 2FA method", Title: "Invalid 2FA method"}
	ErrMfaMethodNotFound   = CustomError{Name: "ErrMfaMethodNotFound", Code: http.StatusNotFound, Message: "MFA method not found", Title: "MFA method not found"}
	ErrMfaMethodNotEnabled = CustomError{Name: "ErrMfaMethodNotEnabled", Code: http.StatusBadRequest, Message: "MFA method not enabled", Title: "MFA method not enabled"}

	ErrOAuthProviderNotFound  = CustomError{Name: "ErrOAuthProviderNotFound", Code: http.StatusNotFound, Message: "OAuth provider not found", Title: "OAuth provider not found"}
	ErrInvalidOAuthState      = CustomError{Name: "ErrInvalidOAuthState", Code: http.StatusBadRequest, Message: "Invalid OAuth state", Title: "Invalid OAuth state"}
	ErrInvalidOAuthProviderID = CustomError{Name: "ErrInvalidOAuthProviderID", Code: http.StatusBadRequest, Message: "Invalid OAuth provider ID", Title: "Invalid OAuth provider ID"}
	ErrInvalidOAuthCode       = CustomError{Name: "ErrInvalidOAuthCode", Code: http.StatusBadRequest, Message: "Invalid OAuth code", Title: "Invalid OAuth code"}

	ErrOAuthProviderMismatch = CustomError{Name: "ErrOAuthProviderMismatch", Code: http.StatusBadRequest, Message: "OAuth provider mismatch for the current user", Title: "OAuth provider mismatch"}

	ErrUserCredentialsNotFound = CustomError{Name: "ErrUserCredentialsNotFound", Code: http.StatusNotFound, Message: "User credentials not found", Title: "User credentials not found"}

	ErrInvalidCodeChallenge = CustomError{Name: "ErrInvalidCodeChallenge", Code: http.StatusBadRequest, Message: "Invalid code challenge", Title: "Invalid code challenge"}
)

var ErrorsList = map[string]CustomError{
	"ErrUserNotFound":                        ErrUserNotFound,
	"ErrEmailOrPasswordInvalid":              ErrEmailOrPasswordInvalid,
	"ErrInvalidEmail":                        ErrInvalidEmail,
	"ErrEmailNotConfirmed":                   ErrEmailNotConfirmed,
	"ErrUserNotActive":                       ErrUserNotActive,
	"ErrUserAlreadyExists":                   ErrUserAlreadyExists,
	"ErrMfaAuthAppNotEnabled":                ErrMfaAuthAppNotEnabled,
	"ErrEmailConfirmationIsInCoolDown":       ErrEmailConfirmationIsInCoolDown,
	"ErrEmailConfirmationNotFound":           ErrEmailConfirmationNotFound,
	"ErrConfirmationTokenAlreadyExpired":     ErrConfirmationTokenAlreadyExpired,
	"ErrConfirmationTokenAlreadyUsed":        ErrConfirmationTokenAlreadyUsed,
	"ErrConfirmationTokenInvalid":            ErrConfirmationTokenInvalid,
	"ErrPasswordResetNotFound":               ErrPasswordResetNotFound,
	"ErrPasswordResetTokenMismatch":          ErrPasswordResetTokenMismatch,
	"ErrPasswordResetTokenExpired":           ErrPasswordResetTokenExpired,
	"ErrInvalidHash":                         ErrInvalidHash,
	"ErrIncompatibleVersion":                 ErrIncompatibleVersion,
	"ErrUserSignUpWithSocial":                ErrUserSignUpWithSocial,
	"ErrApplicationNotFound":                 ErrApplicationNotFound,
	"ErrAplicationSecretNotFound":            ErrAplicationSecretNotFound,
	"ErrOrganizationNotFound":                ErrOrganizationNotFound,
	"ErrUserRoleNotFound":                    ErrUserRoleNotFound,
	"ErrAuthorizationCodeNotFound":           ErrAuthorizationCodeNotFound,
	"ErrAuthorizationCodeExpired":            ErrAuthorizationCodeExpired,
	"ErrInvalidClientSecret":                 ErrInvalidClientSecret,
	"ErrClientSecretExpired":                 ErrClientSecretExpired,
	"ErrAuthorizationCodeInvalidRedirectURI": ErrAuthorizationCodeInvalidRedirectURI,
	"ErrAuthorizationCodeInvalidClientID":    ErrAuthorizationCodeInvalidClientID,
	"ErrAuthorizationCodeInvalidPKCE":        ErrAuthorizationCodeInvalidPKCE,
	"ErrSessionCodeNotFound":                 ErrSessionCodeNotFound,
	"ErrSessionCodeExpired":                  ErrSessionCodeExpired,
	"ErrSessionCodeAlreadyUsed":              ErrSessionCodeAlreadyUsed,
	"ErrMfaEmailNotEnabled":                  ErrMfaEmailNotEnabled,
	"ErrEmailMfaCodeExpired":                 ErrEmailMfaCodeExpired,
	"ErrEmailMfaCodeNotFound":                ErrEmailMfaCodeNotFound,
	"ErrAppMfaCodeNotFound":                  ErrAppMfaCodeNotFound,
	"ErrAppMfaCodeExpired":                   ErrAppMfaCodeExpired,
	"ErrMfaAppNotEnabled":                    ErrMfaAppNotEnabled,
	"ErrChangePasswordCodeNotFound":          ErrChangePasswordCodeNotFound,
	"ErrChangePasswordCodeExpired":           ErrChangePasswordCodeExpired,
	"ErrChangePasswordTokenMismatch":         ErrChangePasswordTokenMismatch,
	"ErrUserShouldNotChangePassword":         ErrUserShouldNotChangePassword,
	"ErrUserShouldChangePassword":            ErrUserShouldChangePassword,
	"ErrMfaUserSecretNotFound":               ErrMfaUserSecretNotFound,
	"ErrInvalidMfaAuthAppCode":               ErrInvalidMfaAuthAppCode,
	"ErrMfaUserSecretAlreadyValidated":       ErrMfaUserSecretAlreadyValidated,
	"ErrMfaCodeNotFound":                     ErrMfaCodeNotFound,
	"ErrMfaCodeExpired":                      ErrMfaCodeExpired,
	"ErrMfaCodeRequired":                     ErrMfaCodeRequired,
	"ErrInvalid2FAMethod":                    ErrInvalid2FAMethod,
	"ErrMfaMethodNotFound":                   ErrMfaMethodNotFound,
	"ErrMfaMethodNotEnabled":                 ErrMfaMethodNotEnabled,
	"ErrOAuthProviderNotFound":               ErrOAuthProviderNotFound,
	"ErrInvalidOAuthState":                   ErrInvalidOAuthState,
	"ErrInvalidOAuthProviderID":              ErrInvalidOAuthProviderID,
	"ErrInvalidOAuthCode":                    ErrInvalidOAuthCode,
	"ErrOAuthProviderMismatch":               ErrOAuthProviderMismatch,
	"ErrUserCredentialsNotFound":             ErrUserCredentialsNotFound,
	"ErrInvalidCodeChallenge":                ErrInvalidCodeChallenge,
}
