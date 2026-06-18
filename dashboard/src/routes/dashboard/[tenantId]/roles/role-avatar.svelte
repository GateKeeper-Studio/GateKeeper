<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { TenantRoleView } from '$lib/services/admin/roles';
	import { roleInitials, roleTint } from './display.js';

	type Props = {
		role: TenantRoleView;
		class?: string;
	};

	let { role, class: className }: Props = $props();
</script>

<span
	class={cn(
		'flex size-9 shrink-0 items-center justify-center rounded-lg text-xs font-semibold',
		roleTint(role),
		className
	)}
	aria-hidden="true"
>
	{roleInitials(role)}
</span>
