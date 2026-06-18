<script lang="ts">
	import SunIcon from '@lucide/svelte/icons/sun';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import { setMode } from 'mode-watcher';
	import { tick } from 'svelte';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	let triggerRef = $state<HTMLElement | null>(null);

	// Telegram-style circular reveal: expand the incoming theme as a growing
	// circle from the toggle button. Falls back to an instant switch when the
	// View Transitions API is unavailable or the user prefers reduced motion.
	async function selectMode(next: 'light' | 'dark' | 'system') {
		const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		if (!('startViewTransition' in document) || reduce || !triggerRef) {
			setMode(next);
			return;
		}

		const rect = triggerRef.getBoundingClientRect();
		const x = rect.left + rect.width / 2;
		const y = rect.top + rect.height / 2;
		const endRadius = Math.hypot(
			Math.max(x, window.innerWidth - x),
			Math.max(y, window.innerHeight - y)
		);

		// Apply the new mode inside the transition and flush mode-watcher's
		// effect (which adds/removes the `.dark` class) before the new snapshot
		// is captured.
		const transition = document.startViewTransition(async () => {
			setMode(next);
			await tick();
		});
		await transition.ready;

		document.documentElement.animate(
			{
				clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${endRadius}px at ${x}px ${y}px)`]
			},
			{
				duration: 500,
				easing: 'ease-in-out',
				pseudoElement: '::view-transition-new(root)'
			}
		);
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button bind:ref={triggerRef} {...props} variant="outline" size="icon">
				<SunIcon
					class="h-[1.2rem] w-[1.2rem] scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90"
				/>
				<MoonIcon
					class="absolute h-[1.2rem] w-[1.2rem] scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0"
				/>
				<span class="sr-only">Toggle theme</span>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		<DropdownMenu.Item onclick={() => selectMode('light')}>Light</DropdownMenu.Item>
		<DropdownMenu.Item onclick={() => selectMode('dark')}>Dark</DropdownMenu.Item>
		<DropdownMenu.Item onclick={() => selectMode('system')}>System</DropdownMenu.Item>
	</DropdownMenu.Content>
</DropdownMenu.Root>
