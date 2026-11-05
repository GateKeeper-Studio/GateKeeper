// Searchable entity sources for the command palette.
//
// Each provider fetches one page (up to 100) of an entity and maps it to
// `SearchResult`s; the palette does the text matching/ranking client-side. The list
// services expose no server-side search param, so a single page is the practical unit.
//
// To make a new entity searchable, add one `SearchProvider` object here — nothing in
// the palette component changes.

import UserIcon from '@lucide/svelte/icons/user';
import GroupIcon from '@lucide/svelte/icons/group';
import UserKeyIcon from '@lucide/svelte/icons/user-key';
import KeyRoundIcon from '@lucide/svelte/icons/key-round';
import LayersIcon from '@lucide/svelte/icons/layers';

import { listTenantUsers, type TenantUserView } from '$lib/services/admin/users';
import { listTenantGroups } from '$lib/services/admin/groups';
import { listTenantRoles } from '$lib/services/admin/roles';
import { listOAuthClients } from '$lib/services/admin/oauth-clients';
import { listTenants } from '$lib/services/admin/tenants';

import type { SearchProvider } from './types';

// The label we show for a user: explicit display name, then "First Last", finally the
// email's local part. Mirrors `userFullName` in the users route (kept inline so this
// lib module doesn't import from `routes/`).
function userLabel(user: TenantUserView): string {
	const display = user.display_name?.trim();
	if (display) return display;
	const composed = `${user.first_name} ${user.last_name}`.trim();
	if (composed) return composed;
	return user.email.split('@')[0];
}

export const searchProviders: SearchProvider[] = [
	{
		key: 'users',
		heading: 'Users',
		icon: UserIcon,
		requiresTenant: true,
		async load(tenantId) {
			if (!tenantId) return [];
			const [data, err] = await listTenantUsers(tenantId, { pageSize: 100 });
			if (err) throw err;
			return data.items.map((u) => ({
				id: u.user_id,
				label: userLabel(u),
				sublabel: u.email,
				href: `/tenants/${tenantId}/users/${u.user_id}`
			}));
		}
	},
	{
		key: 'groups',
		heading: 'Groups',
		icon: GroupIcon,
		requiresTenant: true,
		async load(tenantId) {
			if (!tenantId) return [];
			const [data, err] = await listTenantGroups(tenantId, { pageSize: 100 });
			if (err) throw err;
			return data.items.map((g) => ({
				id: g.group_id,
				label: g.name,
				sublabel: g.description ?? undefined,
				href: `/tenants/${tenantId}/groups/${g.group_id}`
			}));
		}
	},
	{
		key: 'roles',
		heading: 'Roles',
		icon: UserKeyIcon,
		requiresTenant: true,
		async load(tenantId) {
			if (!tenantId) return [];
			const [data, err] = await listTenantRoles(tenantId, { pageSize: 100 });
			if (err) throw err;
			return data.items.map((r) => ({
				id: r.role_id,
				label: r.name,
				sublabel: r.description ?? undefined,
				href: `/tenants/${tenantId}/roles/${r.role_id}`
			}));
		}
	},
	{
		key: 'oauth-clients',
		heading: 'OAuth Clients',
		icon: KeyRoundIcon,
		requiresTenant: true,
		async load(tenantId) {
			if (!tenantId) return [];
			const [data, err] = await listOAuthClients(tenantId);
			if (err) throw err;
			return data.items.map((c) => ({
				// The internal `id` powers the detail route, not the public `client_id`.
				id: c.id,
				label: c.name,
				sublabel: c.client_type,
				href: `/tenants/${tenantId}/oauth-clients/${c.id}`
			}));
		}
	},
	{
		key: 'tenants',
		heading: 'Tenants',
		icon: LayersIcon,
		// Tenants are the top-level scope, so this works even on org-level pages and
		// lets the user jump straight into any tenant.
		requiresTenant: false,
		async load() {
			const [data, err] = await listTenants();
			if (err) throw err;
			return data.items.map((t) => ({
				id: t.tenantId,
				label: t.name,
				sublabel: t.description ?? undefined,
				href: `/tenants/${t.tenantId}`
			}));
		}
	}
];
