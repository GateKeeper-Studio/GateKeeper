<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { MfaMethod } from '../types.js';
  import {
    createMfaClient,
    defaultMfaMethodLabels,
    type MfaClient,
    type MfaConfigurationClasses,
    type MfaConfigurationState,
    type MfaMethodEntry,
    type TotpEnrollment
  } from './mfa.js';

  type Props = {
    initialMfa: MfaConfigurationState;
    client?: MfaClient;
    basePath?: string;
    labels?: Partial<Record<MfaMethod, string>>;
    classes?: MfaConfigurationClasses;
    confirmRemove?: (entry: MfaMethodEntry) => boolean | Promise<boolean>;
    promptRename?: (entry: MfaMethodEntry) => string | null | Promise<string | null>;
    onChange?: (mfa: MfaConfigurationState) => void;
    children?: Snippet;
  };

  let {
    initialMfa,
    client,
    basePath = '/api/mfa',
    labels = {},
    classes = {},
    confirmRemove,
    promptRename,
    onChange
  }: Props = $props();

  const methodLabels = $derived({ ...defaultMfaMethodLabels, ...labels });
  const mfaClient = $derived(client ?? createMfaClient({ basePath }));

  let loadedMfa = $state<MfaConfigurationState | null>(null);
  let mfa = $derived(loadedMfa ?? initialMfa);
  let errorMessage = $state('');
  let busy = $state(false);
  let totpEnrollment = $state<TotpEnrollment | null>(null);
  let totpCode = $state('');
  let addingPasskey = $state(false);
  let passkeyName = $state('');

  let hasConfirmedTotp = $derived(mfa.methods.some((m) => m.method === 'totp' && m.confirmed));

  function cx(...names: Array<string | false | null | undefined>): string {
    return names.filter(Boolean).join(' ');
  }

  function methodLabel(entry: MfaMethodEntry): string {
    return entry.name || methodLabels[entry.method] || entry.method;
  }

  function methodMeta(entry: MfaMethodEntry): string {
    if (!entry.confirmed) return 'Pending confirmation';
    if (entry.lastUsedAt) return `Last used ${new Date(entry.lastUsedAt).toLocaleString()}`;
    if (entry.createdAt) return `Added ${new Date(entry.createdAt).toLocaleDateString()}`;
    return 'Always available';
  }

  async function reload() {
    loadedMfa = await mfaClient.listMethods();
    onChange?.(loadedMfa);
  }

  $effect(() => {
    initialMfa;
    loadedMfa = null;
  });

  async function run(action: () => Promise<void>) {
    errorMessage = '';
    busy = true;
    try {
      await action();
      await reload();
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function setDefault(method: MfaMethod) {
    run(() => mfaClient.setPreferredMethod(method));
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

  function cancelTotp() {
    totpEnrollment = null;
    totpCode = '';
  }

  function addPasskey(event: SubmitEvent) {
    event.preventDefault();
    run(async () => {
      await mfaClient.addPasskey(passkeyName);
      addingPasskey = false;
      passkeyName = '';
    });
  }

  async function removeMethod(entry: MfaMethodEntry) {
    if (!entry.id) return;
    const approved =
      confirmRemove?.(entry) ??
      confirm(`Remove "${methodLabel(entry)}"? You will no longer be able to sign in with it.`);
    if (!(await approved)) return;
    run(() => mfaClient.removeMethod(entry.id as string));
  }

  async function renameMethod(entry: MfaMethodEntry) {
    if (!entry.id) return;
    const name = await (promptRename?.(entry) ?? prompt('New name for this method', entry.name ?? ''));
    if (name === null) return;
    run(() => mfaClient.renameMethod(entry.id as string, name));
  }
</script>

<div class={cx('gk-mfa', classes.root)} data-gatekeeper-mfa>
  {#if errorMessage}
    <p class={cx('gk-mfa__error', classes.error)} role="alert">{errorMessage}</p>
  {/if}

  <section class={cx('gk-mfa__section', classes.section)} data-gatekeeper-mfa-section="methods">
    <h2 class={cx('gk-mfa__title', classes.title)}>Two-factor methods</h2>
    <ul class={cx('gk-mfa__method-list', classes.methodList)}>
      {#each mfa.methods as entry (entry.id ?? entry.method)}
        <li class={cx('gk-mfa__method-row', classes.methodRow)}>
          <div class={cx('gk-mfa__method-info', classes.methodInfo)}>
            <span class={cx('gk-mfa__method-name', classes.methodName)}>
              {methodLabel(entry)}
              {#if entry.name}
                <span class={cx('gk-mfa__muted', classes.muted)}>
                  ({methodLabels[entry.method]})
                </span>
              {/if}
            </span>
            <span class={cx('gk-mfa__method-meta', classes.methodMeta, classes.muted)}>
              {methodMeta(entry)}
            </span>
          </div>
          <div class={cx('gk-mfa__method-actions', classes.methodActions)}>
            {#if entry.confirmed}
              <label class={cx('gk-mfa__default-label', classes.defaultLabel)}>
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
              <button
                type="button"
                class={cx('gk-mfa__button', classes.button)}
                disabled={busy}
                onclick={() => renameMethod(entry)}
              >
                Rename
              </button>
              <button
                type="button"
                class={cx('gk-mfa__button gk-mfa__button--danger', classes.button, classes.dangerButton)}
                disabled={busy}
                onclick={() => removeMethod(entry)}
              >
                Remove
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if mfa.preferredMethod === null}
      <p class={cx('gk-mfa__muted', classes.muted)}>
        No default selected — sign-in picks the strongest available method.
      </p>
    {/if}
  </section>

  <section class={cx('gk-mfa__section', classes.section)} data-gatekeeper-mfa-section="totp">
    <h2 class={cx('gk-mfa__title', classes.title)}>Authenticator app</h2>
    {#if totpEnrollment}
      <p>Scan the QR code or enter the secret manually, then type the 6-digit code.</p>
      <div class={cx('gk-mfa__totp-setup', classes.totpSetup)}>
        {#if totpEnrollment.qrDataUrl}
          <img class={cx('gk-mfa__qr', classes.qr)} src={totpEnrollment.qrDataUrl} alt="TOTP enrollment QR code" />
        {/if}
        {#if totpEnrollment.secretBase32}
          <div class={cx('gk-mfa__totp-secret', classes.totpSecret)}>
            <span class={cx('gk-mfa__muted', classes.muted)}>Manual entry secret</span>
            <code>{totpEnrollment.secretBase32}</code>
            <button
              type="button"
              class={cx('gk-mfa__button', classes.button)}
              onclick={() => navigator.clipboard.writeText(totpEnrollment?.secretBase32 ?? '')}
            >
              Copy secret
            </button>
          </div>
        {/if}
      </div>
      <form class={cx('gk-mfa__actions', classes.actions)} onsubmit={confirmTotp}>
        <input
          class={cx('gk-mfa__input', classes.input)}
          type="text"
          inputmode="numeric"
          autocomplete="one-time-code"
          pattern="[0-9]{6}"
          maxlength="6"
          placeholder="123456"
          bind:value={totpCode}
          required
        />
        <button class={cx('gk-mfa__button gk-mfa__button--primary', classes.button, classes.primaryButton)} type="submit" disabled={busy || totpCode.length !== 6}>
          Confirm
        </button>
        <button class={cx('gk-mfa__button', classes.button)} type="button" disabled={busy} onclick={cancelTotp}>Cancel</button>
      </form>
    {:else if hasConfirmedTotp}
      <p class={cx('gk-mfa__muted', classes.muted)}>
        An authenticator app is enrolled. Remove it above before registering a new one.
      </p>
    {:else}
      <div class={cx('gk-mfa__actions', classes.actions)}>
        <button class={cx('gk-mfa__button gk-mfa__button--primary', classes.button, classes.primaryButton)} type="button" disabled={busy} onclick={startTotp}>
          Set up authenticator app
        </button>
      </div>
    {/if}
  </section>

  <section class={cx('gk-mfa__section', classes.section)} data-gatekeeper-mfa-section="passkeys">
    <h2 class={cx('gk-mfa__title', classes.title)}>Passkeys</h2>
    {#if addingPasskey}
      <form class={cx('gk-mfa__actions', classes.actions)} onsubmit={addPasskey}>
        <input
          class={cx('gk-mfa__input', classes.input)}
          type="text"
          maxlength="100"
          placeholder="Name (e.g. YubiKey 5C)"
          bind:value={passkeyName}
        />
        <button class={cx('gk-mfa__button gk-mfa__button--primary', classes.button, classes.primaryButton)} type="submit" disabled={busy}>Create passkey</button>
        <button class={cx('gk-mfa__button', classes.button)} type="button" disabled={busy} onclick={() => (addingPasskey = false)}>
          Cancel
        </button>
      </form>
    {:else}
      <p class={cx('gk-mfa__muted', classes.muted)}>Register a passkey.</p>
      <div class={cx('gk-mfa__actions', classes.actions)}>
        <button class={cx('gk-mfa__button gk-mfa__button--primary', classes.button, classes.primaryButton)} type="button" disabled={busy} onclick={() => (addingPasskey = true)}>
          Add passkey
        </button>
      </div>
    {/if}
  </section>
</div>
