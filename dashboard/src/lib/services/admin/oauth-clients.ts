import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';
import type { LoginAppearance } from '$lib/types/login-appearance';

// Mirrors the IdP's ClientView (snake_case top-level — that projection has no
// camelCase rename). The nested login_appearance object is camelCase (its view
// struct renames), matching the LoginAppearance type.
export type OAuthClientView = {
	id: string;
	client_id: string;
	name: string;
	client_type: string;
	redirect_uris: string[];
	post_logout_redirect_uris: string[];
	allowed_scopes: string[];
	allowed_grant_types: string[];
	require_pkce: boolean;
	require_consent: boolean;
	mfa_required: boolean;
	mfa_email_enabled: boolean;
	mfa_totp_enabled: boolean;
	mfa_webauthn_enabled: boolean;
	email_provider_id: string | null;
	tos_uri: string | null;
	policy_uri: string | null;
	initiate_login_uri: string | null;
	login_appearance: LoginAppearance | null;
	// When true this client authenticates ONLY through its external identity
	// providers: the IdP refuses every password path for it (register, sign-in,
	// forgot/reset/change), and its hosted login page renders provider buttons only.
	external_login_only: boolean;
	// Magic link login ("email me a sign-in link") is switched on for this client.
	magic_link_enabled: boolean;
	allowed_languages: string[];
	default_language: string;
	is_active: boolean;
	created_at: string;
	updated_at: string | null;
};

type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

export async function listOAuthClients(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<OAuthClientView>>> {
	try {
		const data = await api.get<Paged<OAuthClientView>>(
			`/tenants/${tenantId}/oauth-clients?page=1&page_size=100`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// What the create endpoint accepts (camelCase here, mapped to the IdP's snake_case
// Command below). client_type is set only at creation and is immutable afterwards.
export type CreateOAuthClientInput = {
	name: string;
	clientType: string;
	redirectUris: string[];
	postLogoutRedirectUris: string[];
	allowedScopes: string[];
	allowedGrantTypes: string[];
	requirePkce: boolean;
	requireConsent: boolean;
	mfaRequired: boolean;
	mfaEmailEnabled: boolean;
	mfaTotpEnabled: boolean;
	mfaWebauthnEnabled: boolean;
	emailProviderId: string | null;
	tosUri: string | null;
	policyUri: string | null;
	initiateLoginUri: string | null;
	loginAppearance: LoginAppearance | null;
	// Turning this on requires the client to already have at least one ACTIVE external
	// provider — the IdP rejects the update otherwise, so users can never be locked out.
	externalLoginOnly: boolean;
	magicLinkEnabled: boolean;
	allowedLanguages: string[];
	defaultLanguage: string;
};

// The create response flattens ClientView and adds the raw secret EXACTLY ONCE
// for confidential clients (never retrievable again). Public clients omit it.
export type CreatedOAuthClient = OAuthClientView & { client_secret?: string };

// The editable details of an existing client. client_type is intentionally absent
// (the IdP refuses to change it), is_active is added.
export type UpdateOAuthClientInput = {
	name: string;
	redirectUris: string[];
	postLogoutRedirectUris: string[];
	allowedScopes: string[];
	allowedGrantTypes: string[];
	requirePkce: boolean;
	requireConsent: boolean;
	mfaRequired: boolean;
	mfaEmailEnabled: boolean;
	mfaTotpEnabled: boolean;
	mfaWebauthnEnabled: boolean;
	isActive: boolean;
	emailProviderId: string | null;
	tosUri: string | null;
	policyUri: string | null;
	initiateLoginUri: string | null;
	loginAppearance: LoginAppearance | null;
	// Turning this on requires the client to already have at least one ACTIVE external
	// provider — the IdP rejects the update otherwise, so users can never be locked out.
	externalLoginOnly: boolean;
	magicLinkEnabled: boolean;
	allowedLanguages: string[];
	defaultLanguage: string;
};

export async function createOAuthClient(
	tenantId: string,
	input: CreateOAuthClientInput
): Promise<Result<CreatedOAuthClient>> {
	try {
		const data = await api.post<CreatedOAuthClient>(`/tenants/${tenantId}/oauth-clients`, {
			name: input.name,
			client_type: input.clientType,
			redirect_uris: input.redirectUris,
			post_logout_redirect_uris: input.postLogoutRedirectUris,
			allowed_scopes: input.allowedScopes,
			allowed_grant_types: input.allowedGrantTypes,
			require_pkce: input.requirePkce,
			require_consent: input.requireConsent,
			mfa_required: input.mfaRequired,
			mfa_email_enabled: input.mfaEmailEnabled,
			mfa_totp_enabled: input.mfaTotpEnabled,
			mfa_webauthn_enabled: input.mfaWebauthnEnabled,
			email_provider_id: input.emailProviderId,
			tos_uri: input.tosUri,
			policy_uri: input.policyUri,
			initiate_login_uri: input.initiateLoginUri,
			login_appearance: input.loginAppearance,
			external_login_only: input.externalLoginOnly,
			magic_link_enabled: input.magicLinkEnabled,
			allowed_languages: input.allowedLanguages,
			default_language: input.defaultLanguage
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function updateOAuthClient(
	tenantId: string,
	clientId: string,
	input: UpdateOAuthClientInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/oauth-clients/${clientId}`, {
			name: input.name,
			redirect_uris: input.redirectUris,
			post_logout_redirect_uris: input.postLogoutRedirectUris,
			allowed_scopes: input.allowedScopes,
			allowed_grant_types: input.allowedGrantTypes,
			require_pkce: input.requirePkce,
			require_consent: input.requireConsent,
			mfa_required: input.mfaRequired,
			mfa_email_enabled: input.mfaEmailEnabled,
			mfa_totp_enabled: input.mfaTotpEnabled,
			mfa_webauthn_enabled: input.mfaWebauthnEnabled,
			is_active: input.isActive,
			email_provider_id: input.emailProviderId,
			tos_uri: input.tosUri,
			policy_uri: input.policyUri,
			initiate_login_uri: input.initiateLoginUri,
			login_appearance: input.loginAppearance,
			external_login_only: input.externalLoginOnly,
			magic_link_enabled: input.magicLinkEnabled,
			allowed_languages: input.allowedLanguages,
			default_language: input.defaultLanguage
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function getOAuthClient(
	tenantId: string,
	clientId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<OAuthClientView>> {
	try {
		const data = await api.get<OAuthClientView>(
			`/tenants/${tenantId}/oauth-clients/${clientId}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST …/rotate-secret — issues a fresh secret for a confidential client and
// returns it EXACTLY ONCE (the old secret is invalidated immediately). The IdP
// rejects this with 400 for public clients.
export async function rotateOAuthClientSecret(
	tenantId: string,
	clientId: string
): Promise<Result<CreatedOAuthClient>> {
	try {
		const data = await api.post<CreatedOAuthClient>(
			`/tenants/${tenantId}/oauth-clients/${clientId}/rotate-secret`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function deleteOAuthClient(tenantId: string, clientId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/oauth-clients/${clientId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// Binds (or clears, with null) the SMTP provider a client sends through. The
// update endpoint replaces the editable details, so we resend the client's
// current values from its view and only change email_provider_id.
export async function assignEmailProvider(
	tenantId: string,
	client: OAuthClientView,
	emailProviderId: string | null
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/oauth-clients/${client.id}`, {
			name: client.name,
			redirect_uris: client.redirect_uris,
			post_logout_redirect_uris: client.post_logout_redirect_uris,
			allowed_scopes: client.allowed_scopes,
			allowed_grant_types: client.allowed_grant_types,
			require_pkce: client.require_pkce,
			require_consent: client.require_consent,
			mfa_required: client.mfa_required,
			mfa_email_enabled: client.mfa_email_enabled,
			mfa_totp_enabled: client.mfa_totp_enabled,
			mfa_webauthn_enabled: client.mfa_webauthn_enabled,
			email_provider_id: emailProviderId,
			tos_uri: client.tos_uri,
			policy_uri: client.policy_uri,
			login_appearance: client.login_appearance,
			allowed_languages: client.allowed_languages,
			default_language: client.default_language,
			is_active: client.is_active
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
