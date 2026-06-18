<script lang="ts">
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	let { class: className }: { class?: string } = $props();
</script>

<Loader2Icon class={['size-4 animate-spin', className]} />
