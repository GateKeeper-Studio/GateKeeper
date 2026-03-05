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
	ErrUserProfileNotFound    = CustomError{Name: "ErrUserProfileNotFound", Code: http.StatusInternalServerError, Message: "User profile not found", Title: "User profile not found"}
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

	ErrTenantNotFound = CustomError{Name: "ErrTenantNotFound", Code: http.StatusNotFound, Message: "Tenant not found", Title: "Tenant not found"}

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

	ErrInvalidCodeChallenge       = CustomError{Name: "ErrInvalidCodeChallenge", Code: http.StatusBadRequest, Message: "Invalid code challenge", Title: "Invalid code challenge"}
	ErrInvalidResponseType        = CustomError{Name: "ErrInvalidResponseType", Code: http.StatusBadRequest, Message: "Invalid response_type, only 'code' is supported", Title: "Invalid response type"}
	ErrInvalidCodeChallengeMethod = CustomError{Name: "ErrInvalidCodeChallengeMethod", Code: http.StatusBadRequest, Message: "Invalid code_challenge_method, only 'S256' and 'plain' are supported", Title: "Invalid code challenge method"}

	ErrWebAuthnNotEnabled           = CustomError{Name: "ErrWebAuthnNotEnabled", Code: http.StatusBadRequest, Message: "WebAuthn is not enabled for this user", Title: "WebAuthn not enabled"}
	ErrWebAuthnSessionNotFound      = CustomError{Name: "ErrWebAuthnSessionNotFound", Code: http.StatusNotFound, Message: "WebAuthn session not found", Title: "WebAuthn session not found"}
	ErrWebAuthnSessionExpired       = CustomError{Name: "ErrWebAuthnSessionExpired", Code: http.StatusBadRequest, Message: "WebAuthn session expired", Title: "WebAuthn session expired"}
	ErrWebAuthnCredentialNotFound   = CustomError{Name: "ErrWebAuthnCredentialNotFound", Code: http.StatusNotFound, Message: "WebAuthn credential not found", Title: "WebAuthn credential not found"}
	ErrWebAuthnRegistrationFailed   = CustomError{Name: "ErrWebAuthnRegistrationFailed", Code: http.StatusBadRequest, Message: "WebAuthn registration verification failed", Title: "WebAuthn registration failed"}
	ErrWebAuthnAuthenticationFailed = CustomError{Name: "ErrWebAuthnAuthenticationFailed", Code: http.StatusBadRequest, Message: "WebAuthn authentication verification failed", Title: "WebAuthn authentication failed"}
	ErrWebAuthnNoCredentials        = CustomError{Name: "ErrWebAuthnNoCredentials", Code: http.StatusBadRequest, Message: "User has no registered WebAuthn credentials", Title: "No WebAuthn credentials"}

	// Account / Self-Service Portal errors
	ErrStepUpRequired           = CustomError{Name: "ErrStepUpRequired", Code: http.StatusForbidden, Message: "Step-up authentication is required for this action", Title: "Reauthentication required"}
	ErrStepUpTokenNotFound      = CustomError{Name: "ErrStepUpTokenNotFound", Code: http.StatusNotFound, Message: "Step-up token not found or invalid", Title: "Step-up token not found"}
	ErrStepUpTokenExpired       = CustomError{Name: "ErrStepUpTokenExpired", Code: http.StatusBadRequest, Message: "Step-up token has expired, please reauthenticate", Title: "Step-up token expired"}
	ErrStepUpTokenAlreadyUsed   = CustomError{Name: "ErrStepUpTokenAlreadyUsed", Code: http.StatusBadRequest, Message: "Step-up token has already been used", Title: "Step-up token already used"}
	ErrCurrentPasswordRequired  = CustomError{Name: "ErrCurrentPasswordRequired", Code: http.StatusBadRequest, Message: "Current password is required", Title: "Current password required"}
	ErrCurrentPasswordIncorrect = CustomError{Name: "ErrCurrentPasswordIncorrect", Code: http.StatusBadRequest, Message: "Current password is incorrect", Title: "Incorrect current password"}
	ErrPasswordTooWeak          = CustomError{Name: "ErrPasswordTooWeak", Code: http.StatusBadRequest, Message: "Password does not meet strength requirements: minimum 8 characters, at least one uppercase, one lowercase, one digit, and one special character", Title: "Password too weak"}
	ErrPasswordSameAsCurrent    = CustomError{Name: "ErrPasswordSameAsCurrent", Code: http.StatusBadRequest, Message: "New password must be different from current password", Title: "Same password"}
	ErrMfaAlreadyEnabled        = CustomError{Name: "ErrMfaAlreadyEnabled", Code: http.StatusBadRequest, Message: "MFA is already enabled for this user", Title: "MFA already enabled"}
	ErrMfaNotEnabled            = CustomError{Name: "ErrMfaNotEnabled", Code: http.StatusBadRequest, Message: "MFA is not enabled for this user", Title: "MFA not enabled"}
	ErrBackupCodesNotFound      = CustomError{Name: "ErrBackupCodesNotFound", Code: http.StatusNotFound, Message: "No backup codes found for this user", Title: "Backup codes not found"}
	ErrBackupCodeInvalid        = CustomError{Name: "ErrBackupCodeInvalid", Code: http.StatusBadRequest, Message: "Invalid backup code", Title: "Invalid backup code"}
	ErrEmailChangeNotFound      = CustomError{Name: "ErrEmailChangeNotFound", Code: http.StatusNotFound, Message: "Email change request not found", Title: "Email change not found"}
	ErrEmailChangeExpired       = CustomError{Name: "ErrEmailChangeExpired", Code: http.StatusBadRequest, Message: "Email change request has expired", Title: "Email change expired"}
	ErrEmailAlreadyInUse        = CustomError{Name: "ErrEmailAlreadyInUse", Code: http.StatusConflict, Message: "Email address is already in use", Title: "Email already in use"}
	ErrSessionNotFound          = CustomError{Name: "ErrSessionNotFound", Code: http.StatusNotFound, Message: "Session not found", Title: "Session not found"}
	ErrCannotRevokeCurrentSess  = CustomError{Name: "ErrCannotRevokeCurrentSess", Code: http.StatusBadRequest, Message: "Cannot revoke the current session", Title: "Cannot revoke current session"}
	ErrReauthFailed             = CustomError{Name: "ErrReauthFailed", Code: http.StatusUnauthorized, Message: "Reauthentication failed", Title: "Reauthentication failed"}
)

var ErrorsList = map[string]CustomError{
	"ErrUserNotFound":                        ErrUserNotFound,
	"ErrUserProfileNotFound":                 ErrUserProfileNotFound,
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
	"ErrTenantNotFound":                      ErrTenantNotFound,
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
	"ErrInvalidResponseType":                 ErrInvalidResponseType,
	"ErrInvalidCodeChallengeMethod":          ErrInvalidCodeChallengeMethod,
	"ErrWebAuthnNotEnabled":                  ErrWebAuthnNotEnabled,
	"ErrWebAuthnSessionNotFound":             ErrWebAuthnSessionNotFound,
	"ErrWebAuthnSessionExpired":              ErrWebAuthnSessionExpired,
	"ErrWebAuthnCredentialNotFound":          ErrWebAuthnCredentialNotFound,
	"ErrWebAuthnRegistrationFailed":          ErrWebAuthnRegistrationFailed,
	"ErrWebAuthnAuthenticationFailed":        ErrWebAuthnAuthenticationFailed,
	"ErrWebAuthnNoCredentials":               ErrWebAuthnNoCredentials,
	"ErrStepUpRequired":                      ErrStepUpRequired,
	"ErrStepUpTokenNotFound":                 ErrStepUpTokenNotFound,
	"ErrStepUpTokenExpired":                  ErrStepUpTokenExpired,
	"ErrStepUpTokenAlreadyUsed":              ErrStepUpTokenAlreadyUsed,
	"ErrCurrentPasswordRequired":             ErrCurrentPasswordRequired,
	"ErrCurrentPasswordIncorrect":            ErrCurrentPasswordIncorrect,
	"ErrPasswordTooWeak":                     ErrPasswordTooWeak,
	"ErrPasswordSameAsCurrent":               ErrPasswordSameAsCurrent,
	"ErrMfaAlreadyEnabled":                   ErrMfaAlreadyEnabled,
	"ErrMfaNotEnabled":                       ErrMfaNotEnabled,
	"ErrBackupCodesNotFound":                 ErrBackupCodesNotFound,
	"ErrBackupCodeInvalid":                   ErrBackupCodeInvalid,
	"ErrEmailChangeNotFound":                 ErrEmailChangeNotFound,
	"ErrEmailChangeExpired":                  ErrEmailChangeExpired,
	"ErrEmailAlreadyInUse":                   ErrEmailAlreadyInUse,
	"ErrSessionNotFound":                     ErrSessionNotFound,
	"ErrCannotRevokeCurrentSess":             ErrCannotRevokeCurrentSess,
	"ErrReauthFailed":                        ErrReauthFailed,
}
