// Static navigation targets for the command palette.
//
// One entry per dashboard destination. Org-level pages are always available; tenant-
// scoped pages return `null` when no tenant is selected so the palette hides them
// instead of linking to `/tenants/undefined/...`. Icons mirror the sidebar
// (`app-sidebar/index.svelte`) so the two stay visually consistent.

import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
import Building2Icon from '@lucide/svelte/icons/building-2';
import CircleUserIcon from '@lucide/svelte/icons/circle-user';
import PlusIcon from '@lucide/svelte/icons/plus';
import UserIcon from '@lucide/svelte/icons/user';
import GroupIcon from '@lucide/svelte/icons/group';
import UserKeyIcon from '@lucide/svelte/icons/user-key';
import KeyRoundIcon from '@lucide/svelte/icons/key-round';
import MailsIcon from '@lucide/svelte/icons/mails';
import MailPlusIcon from '@lucide/svelte/icons/mail-plus';
import WebhookIcon from '@lucide/svelte/icons/webhook';
import ShieldIcon from '@lucide/svelte/icons/shield';
import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
import ScrollTextIcon from '@lucide/svelte/icons/scroll-text';
import BookOpenIcon from '@lucide/svelte/icons/book-open';

import { websiteHref } from '$lib/external-urls';
import type { NavCommand } from './types';

// Helper: build a tenant-scoped href, or null when no tenant is active.
const tenantPath =
	(suffix: string) =>
	({ tenantId }: { tenantId?: string }): string | null =>
		tenantId ? `/tenants/${tenantId}/${suffix}` : null;

export const navigationCommands: NavCommand[] = [
	// Organization-level — always available.
	{ id: 'home', label: 'Dashboard Home', icon: LayoutDashboardIcon, keywords: ['overview', 'start'], href: () => '/' },
	{ id: 'organization', label: 'Organization', icon: Building2Icon, keywords: ['org', 'company'], href: () => '/?settings=organization' },
	{ id: 'account', label: 'Account', icon: CircleUserIcon, keywords: ['profile', 'settings', 'me'], href: () => '/?settings=account' },
	{ id: 'create-tenant', label: 'Create Tenant', icon: PlusIcon, keywords: ['new', 'add', 'environment'], href: () => '/tenants/new' },

	// Tenant-scoped — hidden when no tenant is selected.
	{ id: 'users', label: 'Users', icon: UserIcon, keywords: ['people', 'members', 'accounts'], href: tenantPath('users') },
	{ id: 'groups', label: 'Groups', icon: GroupIcon, keywords: ['teams'], href: tenantPath('groups') },
	{ id: 'roles', label: 'Roles', icon: UserKeyIcon, keywords: ['permissions', 'rbac'], href: tenantPath('roles') },
	{ id: 'oauth-clients', label: 'OAuth Clients', icon: KeyRoundIcon, keywords: ['apps', 'applications', 'clients', 'integrations'], href: tenantPath('oauth-clients') },
	{ id: 'email-providers', label: 'E-mail Providers', icon: MailsIcon, keywords: ['smtp', 'mail', 'senders'], href: tenantPath('email-providers') },
	{ id: 'email-templates', label: 'E-mail Templates', icon: MailPlusIcon, keywords: ['mail', 'messages'], href: tenantPath('email-templates') },
	{ id: 'webhooks', label: 'Webhooks', icon: WebhookIcon, keywords: ['events', 'sync', 'integrations', 'notifications'], href: tenantPath('webhooks') },
	{ id: 'audit', label: 'Audit Log', icon: ShieldIcon, keywords: ['events', 'history', 'logs'], href: tenantPath('audit') },
	{ id: 'compliance', label: 'Compliance', icon: ShieldCheckIcon, keywords: ['gdpr', 'privacy'], href: tenantPath('compliance') },
	{ id: 'legal', label: 'Legal Pages', icon: ScrollTextIcon, keywords: ['terms', 'privacy', 'policy'], href: tenantPath('legal') },
	{ id: 'docs', label: 'Documentation', icon: BookOpenIcon, keywords: ['docs', 'help', 'guide', 'tutorial', 'manual'], href: () => websiteHref('/docs') }
];
