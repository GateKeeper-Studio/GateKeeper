<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { TenantUserView } from '$lib/services/admin/users';
	import { userInitials, userTint } from './display.js';

	type Props = {
		user: TenantUserView;
		class?: string;
	};

	let { user, class: className }: Props = $props();
</script>

<span
	class={cn(
		'flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-semibold',
		userTint(user),
		className
	)}
	aria-hidden="true"
>
	{userInitials(user)}
</span>
