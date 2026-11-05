<script lang="ts">
  import {
    createMfaClient,
    defaultMfaMethodLabels,
    type MfaClient,
    type MfaConfigurationState,
    type MfaMethodEntry,
    type MfaMethod,
    type TotpEnrollment
  } from '@gatekeeper/sveltekit-bff/svelte';

  type Props = {
    initialMfa: MfaConfigurationState;
    client?: MfaClient;
    basePath?: string;
  };

  let { initialMfa, client, basePath = '/api/mfa' }: Props = $props();

  const mfaClient = $derived(client ?? createMfaClient({ basePath }));

  let loadedMfa = $state<MfaConfigurationState | null>(null);
  let mfa = $derived(loadedMfa ?? initialMfa);
  let busy = $state(false);
  let errorMessage = $state('');
  let totpEnrollment = $state<TotpEnrollment | null>(null);
  let totpCode = $state('');
  let passkeyName = $state('');

  let hasConfirmedTotp = $derived(mfa.methods.some((entry) => entry.method === 'totp' && entry.confirmed));

  $effect(() => {
    initialMfa;
    loadedMfa = null;
  });

  function methodLabel(entry: MfaMethodEntry): string {
    return entry.name || defaultMfaMethodLabels[entry.method] || entry.method;
  }

  function methodMeta(entry: MfaMethodEntry): string {
    if (!entry.confirmed) return 'Pending confirmation';
    if (entry.lastUsedAt) return `Last used ${new Date(entry.lastUsedAt).toLocaleString()}`;
    if (entry.createdAt) return `Added ${new Date(entry.createdAt).toLocaleDateString()}`;
    return 'Always available';
  }

  async function refresh() {
    loadedMfa = await mfaClient.listMethods();
  }

  async function run(action: () => Promise<void>) {
    errorMessage = '';
    busy = true;
    try {
      await action();
      await refresh();
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function setDefault(method: MfaMethod) {
    run(() => mfaClient.setPreferredMethod(method));
  }

  function renameMethod(entry: MfaMethodEntry) {
    if (!entry.id) return;
    const name = prompt('New name for this method', entry.name ?? '');
    if (name === null) return;
    run(() => mfaClient.renameMethod(entry.id as string, name));
  }

  function removeMethod(entry: MfaMethodEntry) {
    if (!entry.id) return;
    if (!confirm(`Remove "${methodLabel(entry)}"?`)) return;
    run(() => mfaClient.removeMethod(entry.id as string));
  }

  function startTotp() {
    run(async () => {
      totpEnrollment = await mfaClient.startTotpEnrollment();
      totpCode = '';
    });
  }

  function confirmTotp(event: SubmitEvent) {
    event.preventDefault();
    run(async () => {
      await mfaClient.confirmTotpEnrollment(totpCode);
      totpEnrollment = null;
      totpCode = '';
    });
  }

  function addPasskey(event: SubmitEvent) {
    event.preventDefault();
    run(async () => {
      await mfaClient.addPasskey(passkeyName);
      passkeyName = '';
    });
  }
</script>

<section class="card">
  <h2>Custom MFA component</h2>
  <p class="muted">
    This example uses the package headless MFA client directly instead of the default component.
  </p>

  {#if errorMessage}
    <p class="form-error" role="alert">{errorMessage}</p>
  {/if}

  <ul class="method-list">
    {#each mfa.methods as entry (entry.id ?? entry.method)}
      <li class="method-row">
        <div class="method-info">
          <span class="method-name">{methodLabel(entry)}</span>
          <span class="method-meta muted">{methodMeta(entry)}</span>
        </div>
        <div class="method-actions">
          {#if entry.confirmed}
            <label class="check-row default-radio">
              <input
                type="radio"
                name="headless-default-method"
                checked={entry.isPreferred}
                disabled={busy}
                onchange={() => setDefault(entry.method)}
              />
              <span>Default</span>
            </label>
          {/if}
          {#if entry.id}
            <button type="button" disabled={busy} onclick={() => renameMethod(entry)}>Rename</button>
            <button type="button" class="danger" disabled={busy} onclick={() => removeMethod(entry)}>Remove</button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>

  {#if totpEnrollment}
    <div class="totp-setup">
      {#if totpEnrollment.qrDataUrl}
        <img class="qr" src={totpEnrollment.qrDataUrl} alt="TOTP enrollment QR code" />
      {/if}
      {#if totpEnrollment.secretBase32}
        <div class="totp-secret">
          <span class="muted">Manual entry secret</span>
          <code>{totpEnrollment.secretBase32}</code>
        </div>
      {/if}
    </div>
    <form class="actions" onsubmit={confirmTotp}>
      <input
        type="text"
        inputmode="numeric"
        autocomplete="one-time-code"
        pattern="[0-9]{6}"
        maxlength="6"
        placeholder="123456"
        bind:value={totpCode}
        required
      />
      <button class="primary" type="submit" disabled={busy || totpCode.length !== 6}>Confirm</button>
      <button type="button" disabled={busy} onclick={() => (totpEnrollment = null)}>Cancel</button>
    </form>
  {:else if !hasConfirmedTotp}
    <div class="actions">
      <button class="primary" type="button" disabled={busy} onclick={startTotp}>
        Set up authenticator app
      </button>
    </div>
  {/if}

  <form class="actions" onsubmit={addPasskey}>
    <input type="text" maxlength="100" placeholder="Passkey name" bind:value={passkeyName} />
    <button class="primary" type="submit" disabled={busy}>Add passkey</button>
  </form>
</section>
