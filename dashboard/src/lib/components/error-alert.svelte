<script lang="ts">
	import AlertCircleIcon from '@lucide/svelte/icons/circle-alert';
	import * as Alert from '$lib/components/ui/alert/index.js';

	let { title, message }: { title: string; message: string } = $props();
</script>

<Alert.Root variant="destructive" class="bg-red-500/10">
	<AlertCircleIcon class="h-4 w-4" />
	<Alert.Title>{title}</Alert.Title>
	<Alert.Description>{message}</Alert.Description>
</Alert.Root>
