// Presentation helpers for the tenant users page. Pure functions over the IdP's
// TenantUserView projection.
import type { TenantUserView } from '$lib/services/admin/users';

// The name we show as the primary label: prefer the explicit display name, then
// a "First Last" composition, finally the email's local part as a last resort.
export function userFullName(user: TenantUserView): string {
	const display = user.display_name?.trim();
	if (display) return display;

	const composed = `${user.first_name} ${user.last_name}`.trim();
	if (composed) return composed;

	return user.email.split('@')[0];
}

// Up to two uppercase initials for the avatar fallback, derived from whichever
// name we end up showing.
export function userInitials(user: TenantUserView): string {
	const parts = userFullName(user)
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2);

	const initials = parts.map((p) => p[0]).join('');
	return (initials || user.email[0] || '?').toUpperCase();
}

// A deterministic tint per user so avatars feel distinct without storing a color.
// Hashes the user id into one of a small, theme-aware palette.
const AVATAR_TINTS = [
	'bg-sky-500/10 text-sky-600 dark:text-sky-400',
	'bg-violet-500/10 text-violet-600 dark:text-violet-400',
	'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
	'bg-amber-500/10 text-amber-600 dark:text-amber-400',
	'bg-rose-500/10 text-rose-600 dark:text-rose-400',
	'bg-teal-500/10 text-teal-600 dark:text-teal-400',
	'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400'
];

export function userTint(user: TenantUserView): string {
	let hash = 0;
	for (let i = 0; i < user.user_id.length; i++) {
		hash = (hash * 31 + user.user_id.charCodeAt(i)) >>> 0;
	}
	return AVATAR_TINTS[hash % AVATAR_TINTS.length];
}

export function formatUserDate(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return iso;
	return date.toLocaleDateString(undefined, { dateStyle: 'medium' });
}
