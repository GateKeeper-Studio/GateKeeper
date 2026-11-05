<script lang="ts">
  import '../app.css';
  import { GateKeeperProvider } from '@gatekeeper/sveltekit-bff/svelte';
  import type { LayoutData } from './$types';

  let { children, data }: { children: import('svelte').Snippet; data: LayoutData } = $props();
</script>

<GateKeeperProvider initialSession={data.gatekeeper}>
  {@render children()}
</GateKeeperProvider>
