// Registry of settings sections — the decoupling seam of the settings dialog.
//
// The dialog shell (`index.svelte`) renders this array and knows nothing about any
// section's internals. Adding a new section = a new `*-section.svelte` component + one
// entry here. Icons mirror the sidebar / command palette (`@lucide/svelte/icons/*`).

import type { Component } from 'svelte';

import { m } from '$lib/paraglide/messages.js';

import CircleUserIcon from '@lucide/svelte/icons/circle-user';
import Building2Icon from '@lucide/svelte/icons/building-2';
import PaletteIcon from '@lucide/svelte/icons/palette';
import LanguagesIcon from '@lucide/svelte/icons/languages';

import AccountSection from './sections/account-section.svelte';
import OrganizationSection from './sections/organization-section.svelte';
import AppearanceSection from './sections/appearance-section.svelte';
import LanguageSection from './sections/language-section.svelte';

export type SettingsSection = {
	/** Stable id used in `?settings=<id>` deep links and `settingsDialog.section`. */
	id: string;
	/** Rail label. */
	label: string;
	/** Rail icon. */
	icon: Component;
	/** Rail grouping heading; sections keep array order within a group. */
	group: string;
	/** Optional nested screens rendered inside this section. Used for dialog breadcrumbs. */
	screens?: { id: string; label: string }[];
	/** The panel rendered in the right pane when this section is active. */
	component: Component;
};

export const settingsSections: SettingsSection[] = [
	{
		id: 'account',
		label: m.settings_account_title(),
		icon: CircleUserIcon,
		group: m.settings_group_personal(),
		component: AccountSection
	},
	{
		id: 'language',
		label: m.settings_language_title(),
		icon: LanguagesIcon,
		group: m.settings_group_personal(),
		component: LanguageSection
	},
	{
		id: 'organization',
		label: m.settings_organization_title(),
		icon: Building2Icon,
		group: m.settings_group_organization(),
		component: OrganizationSection
	},
	{
		id: 'appearance',
		label: m.settings_appearance_title(),
		icon: PaletteIcon,
		group: m.settings_group_organization(),
		component: AppearanceSection
	}
];

/** Sections grouped by `group`, preserving first-seen order (used to render the rail). */
export const settingsGroups: { name: string; items: SettingsSection[] }[] = (() => {
	const groups: { name: string; items: SettingsSection[] }[] = [];
	for (const section of settingsSections) {
		let group = groups.find((g) => g.name === section.group);
		if (!group) {
			group = { name: section.group, items: [] };
			groups.push(group);
		}
		group.items.push(section);
	}
	return groups;
})();
