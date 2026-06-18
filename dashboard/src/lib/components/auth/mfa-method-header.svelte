<script lang="ts">
	import type { EMfaType } from '$lib/services/auth/login';
	import { mfaMethodMeta } from './mfa-method-meta';

	let { method }: { method: EMfaType } = $props();

	const meta = $derived(mfaMethodMeta[method]);
	const Icon = $derived(meta.icon);
</script>

<div class="flex flex-col items-center space-y-3 text-center">
	<div
		class="flex h-14 w-14 items-center justify-center rounded-full bg-primary/10 text-primary"
		aria-hidden="true"
	>
		<Icon class="h-7 w-7" />
	</div>

	<div class="space-y-1">
		<p class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
			Two-Factor Authentication
		</p>
		<h1 class="text-2xl font-semibold tracking-tight">{meta.heading}</h1>
	</div>

	<p class="text-sm text-muted-foreground">{meta.instruction}</p>
</div>
