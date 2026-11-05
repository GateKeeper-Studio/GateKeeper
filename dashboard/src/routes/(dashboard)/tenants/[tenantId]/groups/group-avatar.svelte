<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { TenantGroupView } from '$lib/services/admin/groups';
	import { groupInitials, groupTint } from './display.js';

	type Props = {
		group: TenantGroupView;
		class?: string;
	};

	let { group, class: className }: Props = $props();
</script>

<span
	class={cn(
		'flex size-9 shrink-0 items-center justify-center rounded-lg text-xs font-semibold',
		groupTint(group),
		className
	)}
	aria-hidden="true"
>
	{groupInitials(group)}
</span>
