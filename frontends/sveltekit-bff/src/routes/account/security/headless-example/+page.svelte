<script lang="ts">
  import { HeadlessMfaConfiguration } from '$lib/components/gatekeeper';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
</script>

<main>
  <header>
    <h1>Headless MFA example</h1>
    <p class="subtitle">
      This page uses a frontend-local component built with
      <code>createMfaClient</code> from <code>@gatekeeper/sveltekit-bff/svelte</code>.
    </p>
  </header>

  <HeadlessMfaConfiguration initialMfa={data.mfa} />

  <div class="actions">
    <a class="button" href="/account/security">Back to default component</a>
  </div>
</main>
