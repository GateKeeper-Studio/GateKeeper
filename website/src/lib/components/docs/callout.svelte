<script lang="ts">
	import type { Snippet } from 'svelte';
	import InfoIcon from '@lucide/svelte/icons/info';
	import LightbulbIcon from '@lucide/svelte/icons/lightbulb';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import OctagonAlertIcon from '@lucide/svelte/icons/octagon-alert';
	import MessageSquareWarningIcon from '@lucide/svelte/icons/message-square-warning';

	import * as Alert from '$lib/components/ui/alert/index.js';

	type Kind = 'note' | 'tip' | 'important' | 'warning' | 'caution';

	let {
		type = 'note',
		title,
		children
	}: { type?: Kind; title?: string; children?: Snippet } = $props();

	const VARIANTS = {
		note: { icon: InfoIcon, label: 'Note', class: 'border-blue-500/40 bg-blue-500/5' },
		tip: { icon: LightbulbIcon, label: 'Tip', class: 'border-emerald-500/40 bg-emerald-500/5' },
		important: {
			icon: MessageSquareWarningIcon,
			label: 'Important',
			class: 'border-violet-500/40 bg-violet-500/5'
		},
		warning: {
			icon: TriangleAlertIcon,
			label: 'Warning',
			class: 'border-amber-500/40 bg-amber-500/5'
		},
		caution: { icon: OctagonAlertIcon, label: 'Caution', class: 'border-red-500/40 bg-red-500/5' }
	} as const;

	const variant = $derived(VARIANTS[type] ?? VARIANTS.note);
	const Icon = $derived(variant.icon);
</script>

<!-- `not-prose` stops typography from restyling the alert; the inner overrides re-enable just
     enough for markdown (paragraphs, inline code) to look right inside it. -->
<Alert.Root class="not-prose my-6 {variant.class}">
	<Icon class="size-4" />
	<Alert.Title>{title ?? variant.label}</Alert.Title>
	<Alert.Description
		class="[&_a]:underline [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:text-xs [&_p]:m-0 [&_p+p]:mt-2"
	>
		{@render children?.()}
	</Alert.Description>
</Alert.Root>
