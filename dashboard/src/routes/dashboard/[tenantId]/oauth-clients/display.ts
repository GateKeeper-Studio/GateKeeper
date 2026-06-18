// Presentation helpers for the OAuth client list + detail pages. Pure functions over
// the IdP's OAuthClientView projection.
import type { BadgeVariant } from '$lib/components/ui/badge/index.js';
import type { OAuthClientView } from '$lib/services/admin/oauth-clients';

export function clientTypeLabel(type: string): string {
	return type === 'confidential' ? 'Confidential' : 'Public';
}

const SCOPE_LABELS: Record<string, string> = {
	openid: 'OpenID',
	profile: 'Profile',
	email: 'Email',
	offline_access: 'Offline access'
};

export function scopeLabel(scope: string): string {
	return SCOPE_LABELS[scope] ?? scope;
}

// The scopes/grants the form offers as checkboxes. `openid` is effectively
// mandatory for OIDC; the rest are opt-in. Anything else can still be added as a
// custom scope on the form.
export const KNOWN_SCOPES = ['openid', 'profile', 'email', 'offline_access'] as const;
export const KNOWN_GRANT_TYPES = ['authorization_code', 'refresh_token', 'client_credentials'] as const;

const GRANT_LABELS: Record<string, string> = {
	authorization_code: 'Authorization Code',
	refresh_token: 'Refresh Token',
	client_credentials: 'Client Credentials'
};

export function grantLabel(grant: string): string {
	return GRANT_LABELS[grant] ?? grant;
}

export function formatClientDate(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return iso;
	return date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export type ClientBadge = { label: string; variant: BadgeVariant };

// A compact set of badges summarising a client at a glance, mirroring the reference
// dashboard's `application.badges`.
export function clientBadges(client: OAuthClientView): ClientBadge[] {
	const badges: ClientBadge[] = [{ label: clientTypeLabel(client.client_type), variant: 'secondary' }];
	if (client.require_pkce) badges.push({ label: 'PKCE', variant: 'outline' });
	if (client.require_consent) badges.push({ label: 'Consent', variant: 'outline' });
	if (client.allowed_grant_types.includes('refresh_token'))
		badges.push({ label: 'Refresh tokens', variant: 'outline' });
	return badges;
}
