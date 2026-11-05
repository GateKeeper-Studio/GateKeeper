import type { TenantGroupView } from '$lib/services/admin/groups';

export function groupInitials(group: TenantGroupView): string {
	const parts = group.name.trim().split(/\s+/).filter(Boolean).slice(0, 2);
	return (parts.map((p) => p[0]).join('') || '?').toUpperCase();
}

const AVATAR_TINTS = [
	'bg-sky-500/10 text-sky-600 dark:text-sky-400',
	'bg-violet-500/10 text-violet-600 dark:text-violet-400',
	'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
	'bg-amber-500/10 text-amber-600 dark:text-amber-400',
	'bg-rose-500/10 text-rose-600 dark:text-rose-400',
	'bg-teal-500/10 text-teal-600 dark:text-teal-400',
	'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400'
];

export function groupTint(group: TenantGroupView): string {
	let hash = 0;
	for (let i = 0; i < group.group_id.length; i++) {
		hash = (hash * 31 + group.group_id.charCodeAt(i)) >>> 0;
	}
	return AVATAR_TINTS[hash % AVATAR_TINTS.length];
}

export function formatGroupDate(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return iso;
	return date.toLocaleDateString(undefined, { dateStyle: 'medium' });
}
