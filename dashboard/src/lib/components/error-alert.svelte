<script lang="ts">
	import AlertCircleIcon from '@lucide/svelte/icons/circle-alert';
	import * as Alert from '$lib/components/ui/alert/index.js';

	let { title, message }: { title: string; message: string } = $props();
</script>

<Alert.Root variant="destructive" class="bg-red-500/10">
	<AlertCircleIcon class="h-4 w-4" />
	<Alert.Title class="font-bold">{title}</Alert.Title>
	<Alert.Description class="dark:text-gray-100">{message}</Alert.Description>
</Alert.Root>
