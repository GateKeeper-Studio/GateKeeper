<script>
  // Self-service MFA management. Every mutation goes through our /api/mfa/*
  // proxy routes (BFF): the browser never holds the IdP access token.
  import { registerPasskey } from '$lib/webauthn.js';

  let { data } = $props();

  const methodLabels = {
    email: 'Email code',
    totp: 'Authenticator app',
    webauthn: 'Passkey'
  };

  let mfa = $state(data.mfa);
  let errorMessage = $state('');
  let busy = $state(false);

  // Authenticator-app enrollment panel ({ enrollmentId, secretBase32, qrDataUrl })
  // shown between "start" and "confirm".
  let totpEnrollment = $state(null);
  let totpCode = $state('');

  let addingPasskey = $state(false);
  let passkeyName = $state('');

  let hasConfirmedTotp = $derived(
    mfa.methods.some((m) => m.method === 'totp' && m.confirmed)
  );

  async function api(path, init = {}) {
    const res = await fetch(`/api/mfa${path}`, {
      ...init,
      headers: init.body ? { 'Content-Type': 'application/json' } : undefined
    });
    if (!res.ok) {
      const detail = await res.json().catch(() => null);
      throw new Error(detail?.message ?? detail?.error ?? `Request failed (${res.status})`);
    }
    return res.status === 204 ? null : res.json();
  }

  async function run(action) {
    errorMessage = '';
    busy = true;
    try {
      await action();
      mfa = await api('/methods');
    } catch (err) {
      errorMessage = err.message;
    } finally {
      busy = false;
    }
  }

  function setDefault(method) {
    run(() => api('/preferred', { method: 'PUT', body: JSON.stringify({ method }) }));
  }

  function startTotp() {
    run(async () => {
      totpEnrollment = await api('/totp', { method: 'POST' });
      totpCode = '';
    });
  }

  function confirmTotp(event) {
    event.preventDefault();
    run(async () => {
      await api('/totp/confirm', { method: 'POST', body: JSON.stringify({ code: totpCode }) });
      totpEnrollment = null;
      totpCode = '';
    });
  }

  function cancelTotp() {
    totpEnrollment = null;
    totpCode = '';
  }

  function addPasskey(event) {
    event.preventDefault();
    run(async () => {
      const { mfaId, options } = await api('/webauthn/start', { method: 'POST' });
      const attestation = await registerPasskey(options);
      await api('/webauthn/finish', {
        method: 'POST',
        body: JSON.stringify({ mfaId, name: passkeyName.trim() || null, attestation })
      });
      addingPasskey = false;
      passkeyName = '';
    });
  }

  function removeMethod(entry) {
    const label = entry.name || methodLabels[entry.method];
    if (!confirm(`Remove "${label}"? You will no longer be able to sign in with it.`)) return;
    run(() => api(`/methods/${entry.id}`, { method: 'DELETE' }));
  }

  function renameMethod(entry) {
    const name = prompt('New name for this method', entry.name ?? '');
    if (name === null) return;
    run(() => api(`/methods/${entry.id}`, { method: 'PATCH', body: JSON.stringify({ name }) }));
  }
</script>

<main>
  <header>
    <h1>Security settings</h1>
    <p class="subtitle">
      Manage the second factors for <strong>{data.user?.email ?? data.user?.sub}</strong>. The
      default method is challenged first on your next sign-in.
    </p>
  </header>

  {#if errorMessage}
    <p class="form-error" role="alert">{errorMessage}</p>
  {/if}

  <section class="card">
    <h2>Two-factor methods</h2>
    <ul class="method-list">
      {#each mfa.methods as entry (entry.id ?? entry.method)}
        <li class="method-row">
          <div class="method-info">
            <span class="method-name">
              {entry.name || methodLabels[entry.method] || entry.method}
              {#if entry.name}<span class="muted">({methodLabels[entry.method]})</span>{/if}
            </span>
            <span class="method-meta muted">
              {#if !entry.confirmed}
                Pending confirmation
              {:else if entry.lastUsedAt}
                Last used {new Date(entry.lastUsedAt).toLocaleString()}
              {:else if entry.createdAt}
                Added {new Date(entry.createdAt).toLocaleDateString()}
              {:else}
                Always available
              {/if}
            </span>
          </div>
          <div class="method-actions">
            {#if entry.confirmed}
              <label class="check-row default-radio">
                <input
                  type="radio"
                  name="default-method"
                  checked={entry.isPreferred}
                  disabled={busy}
                  onchange={() => setDefault(entry.method)}
                />
                <span>Default</span>
              </label>
            {/if}
            {#if entry.id}
              <button type="button" disabled={busy} onclick={() => renameMethod(entry)}>
                Rename
              </button>
              <button type="button" class="danger" disabled={busy} onclick={() => removeMethod(entry)}>
                Remove
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if mfa.preferredMethod === null}
      <p class="muted">
        No default selected — sign-in picks the strongest available method (passkey, then
        authenticator app, then email).
      </p>
    {/if}
  </section>

  <section class="card">
    <h2>Authenticator app</h2>
    {#if totpEnrollment}
      <p>
        Scan the QR code with your authenticator app (or enter the secret manually), then type
        the 6-digit code it shows to finish.
      </p>
      <div class="totp-setup">
        <img class="qr" src={totpEnrollment.qrDataUrl} alt="TOTP enrollment QR code" />
        <div class="totp-secret">
          <span class="muted">Manual entry secret</span>
          <code>{totpEnrollment.secretBase32}</code>
          <button
            type="button"
            onclick={() => navigator.clipboard.writeText(totpEnrollment.secretBase32)}
          >
            Copy secret
          </button>
        </div>
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
        <button class="primary" type="submit" disabled={busy || totpCode.length !== 6}>
          Confirm
        </button>
        <button type="button" disabled={busy} onclick={cancelTotp}>Cancel</button>
      </form>
    {:else if hasConfirmedTotp}
      <p class="muted">
        An authenticator app is enrolled. Remove it above before registering a new one.
      </p>
    {:else}
      <div class="actions">
        <button class="primary" type="button" disabled={busy} onclick={startTotp}>
          Set up authenticator app
        </button>
      </div>
    {/if}
  </section>

  <section class="card">
    <h2>Passkeys</h2>
    {#if addingPasskey}
      <form class="actions" onsubmit={addPasskey}>
        <input
          type="text"
          maxlength="100"
          placeholder="Name (e.g. YubiKey 5C)"
          bind:value={passkeyName}
        />
        <button class="primary" type="submit" disabled={busy}>Create passkey</button>
        <button type="button" disabled={busy} onclick={() => (addingPasskey = false)}>
          Cancel
        </button>
      </form>
    {:else}
      <p class="muted">Register a passkey (security key, fingerprint, or device PIN).</p>
      <div class="actions">
        <button class="primary" type="button" disabled={busy} onclick={() => (addingPasskey = true)}>
          Add passkey
        </button>
      </div>
    {/if}
  </section>

  <div class="actions">
    <a class="button" href="/">Back to home</a>
  </div>
</main>
