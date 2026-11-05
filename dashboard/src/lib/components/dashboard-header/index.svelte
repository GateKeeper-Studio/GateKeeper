<script lang="ts">
	import Breadcrumbs from './breadcrumbs.svelte';
	import SearchCommand from './search-command.svelte';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';

	type BreadcrumbItem = {
		name: string;
		path?: string;
	};

	let {
		breadcrumbs
	}: {
		breadcrumbs: {
			items: BreadcrumbItem[];
			disableSideBar?: boolean;
		};
	} = $props();
</script>

<header
	class="flex w-full items-center gap-4 border-b border-gray-200 px-2 py-1 dark:border-gray-700"
>
	<Breadcrumbs {...breadcrumbs} />
	<SearchCommand />
	<LanguageSwitcher />
	<ThemeToggle />
</header>
