import { createMfaClient, defaultMfaMethodLabels } from '../mfa.js';
import type {
  MfaClient,
  MfaConfigurationState,
  MfaMethod,
  MfaMethodEntry,
  TotpEnrollment
} from '../types.js';

function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function parseInitialMfa(value: string | null): MfaConfigurationState | null {
  if (!value) return null;
  try {
    return JSON.parse(value) as MfaConfigurationState;
  } catch {
    return null;
  }
}

function parseLabels(value: string | null): Partial<Record<MfaMethod, string>> {
  if (!value) return {};
  try {
    return JSON.parse(value) as Partial<Record<MfaMethod, string>>;
  } catch {
    return {};
  }
}

function methodKey(entry: MfaMethodEntry): string {
  return entry.id ?? entry.method;
}

export class GateKeeperMfaConfigurationElement extends HTMLElement {
  static observedAttributes = ['base-path', 'initial-mfa', 'labels'];

  #client: MfaClient | null = null;
  #initialMfa: MfaConfigurationState | null | undefined;
  #state: MfaConfigurationState = { methods: [], preferredMethod: null };
  #labels: Partial<Record<MfaMethod, string>> = {};
  #errorMessage = '';
  #busy = false;
  #totpEnrollment: TotpEnrollment | null = null;
  #addingPasskey = false;

  get client(): MfaClient {
    if (!this.#client) {
      this.#client = createMfaClient({ basePath: this.basePath });
    }
    return this.#client;
  }

  set client(client: MfaClient) {
    this.#client = client;
  }

  get initialMfa(): MfaConfigurationState | null | undefined {
    return this.#initialMfa;
  }

  set initialMfa(state: MfaConfigurationState | null | undefined) {
    this.#initialMfa = state;
    if (state) this.#state = state;
    this.render();
  }

  get labels(): Partial<Record<MfaMethod, string>> {
    return this.#labels;
  }

  set labels(labels: Partial<Record<MfaMethod, string>>) {
    this.#labels = labels;
    this.render();
  }

  get basePath(): string {
    return this.getAttribute('base-path') ?? '/api/mfa';
  }

  connectedCallback() {
    this.addEventListener('click', this.#handleClick);
    this.addEventListener('submit', this.#handleSubmit);

    const initialMfa = this.#initialMfa ?? parseInitialMfa(this.getAttribute('initial-mfa'));
    if (initialMfa) {
      this.#state = initialMfa;
      this.render();
      return;
    }

    this.render();
    this.reload().catch((error: unknown) => this.#setError(error));
  }

  disconnectedCallback() {
    this.removeEventListener('click', this.#handleClick);
    this.removeEventListener('submit', this.#handleSubmit);
  }

  attributeChangedCallback(name: string) {
    if (name === 'base-path') {
      this.#client = null;
    } else if (name === 'initial-mfa') {
      const initialMfa = parseInitialMfa(this.getAttribute('initial-mfa'));
      if (initialMfa) this.#state = initialMfa;
    } else if (name === 'labels') {
      this.#labels = parseLabels(this.getAttribute('labels'));
    }

    if (this.isConnected) this.render();
  }

  async reload(): Promise<MfaConfigurationState> {
    const state = await this.client.listMethods();
    this.#state = state;
    this.#emitChange();
    this.render();
    return state;
  }

  render() {
    const labels = { ...defaultMfaMethodLabels, ...this.#labels };
    const hasConfirmedTotp = this.#state.methods.some((method) => method.method === 'totp' && method.confirmed);
    const methods = this.#state.methods
      .map((entry) => this.#renderMethod(entry, labels))
      .join('');

    this.innerHTML = `
      <div class="gk-mfa" data-gatekeeper-mfa>
        ${this.#errorMessage ? `<p class="gk-mfa__error" role="alert">${escapeHtml(this.#errorMessage)}</p>` : ''}
        <section class="gk-mfa__section" data-gatekeeper-mfa-section="methods">
          <h2 class="gk-mfa__title">Two-factor methods</h2>
          <ul class="gk-mfa__method-list">${methods}</ul>
          ${
            this.#state.preferredMethod === null
              ? '<p class="gk-mfa__muted">No default selected. Sign-in picks the strongest available method.</p>'
              : ''
          }
        </section>
        ${this.#renderTotpSection(hasConfirmedTotp)}
        ${this.#renderPasskeySection()}
      </div>
    `;
  }

  #renderMethod(entry: MfaMethodEntry, labels: Record<MfaMethod, string>): string {
    const label = entry.name || labels[entry.method] || entry.method;
    const meta = this.#methodMeta(entry);
    const key = escapeHtml(methodKey(entry));
    const renameRemove =
      entry.id === undefined
        ? ''
        : `
          <button type="button" class="gk-mfa__button" data-action="rename" data-method-key="${key}" ${this.#disabledAttr()}>
            Rename
          </button>
          <button type="button" class="gk-mfa__button gk-mfa__button--danger" data-action="remove" data-method-key="${key}" ${this.#disabledAttr()}>
            Remove
          </button>
        `;

    return `
      <li class="gk-mfa__method-row" data-method-key="${key}">
        <div class="gk-mfa__method-info">
          <span class="gk-mfa__method-name">
            ${escapeHtml(label)}
            ${entry.name ? `<span class="gk-mfa__muted">(${escapeHtml(labels[entry.method])})</span>` : ''}
          </span>
          <span class="gk-mfa__method-meta gk-mfa__muted">${escapeHtml(meta)}</span>
        </div>
        <div class="gk-mfa__method-actions">
          ${
            entry.confirmed
              ? `
                <label class="gk-mfa__default-label">
                  <input type="radio" name="default-method" data-action="default" data-method="${entry.method}" ${entry.isPreferred ? 'checked' : ''} ${this.#disabledAttr()} />
                  <span>Default</span>
                </label>
              `
              : ''
          }
          ${renameRemove}
        </div>
      </li>
    `;
  }

  #renderTotpSection(hasConfirmedTotp: boolean): string {
    if (this.#totpEnrollment) {
      return `
        <section class="gk-mfa__section" data-gatekeeper-mfa-section="totp">
          <h2 class="gk-mfa__title">Authenticator app</h2>
          <p>Scan the QR code or enter the secret manually, then type the 6-digit code.</p>
          <div class="gk-mfa__totp-setup">
            ${
              this.#totpEnrollment.qrDataUrl
                ? `<img class="gk-mfa__qr" src="${escapeHtml(this.#totpEnrollment.qrDataUrl)}" alt="TOTP enrollment QR code" />`
                : ''
            }
            ${
              this.#totpEnrollment.secretBase32
                ? `
                  <div class="gk-mfa__totp-secret">
                    <span class="gk-mfa__muted">Manual entry secret</span>
                    <code>${escapeHtml(this.#totpEnrollment.secretBase32)}</code>
                    <button type="button" class="gk-mfa__button" data-action="copy-totp-secret">Copy secret</button>
                  </div>
                `
                : ''
            }
          </div>
          <form class="gk-mfa__actions" data-action="confirm-totp">
            <input class="gk-mfa__input" name="totp-code" type="text" inputmode="numeric" autocomplete="one-time-code" pattern="[0-9]{6}" maxlength="6" placeholder="123456" required />
            <button class="gk-mfa__button gk-mfa__button--primary" type="submit" ${this.#disabledAttr()}>Confirm</button>
            <button class="gk-mfa__button" type="button" data-action="cancel-totp" ${this.#disabledAttr()}>Cancel</button>
          </form>
        </section>
      `;
    }

    return `
      <section class="gk-mfa__section" data-gatekeeper-mfa-section="totp">
        <h2 class="gk-mfa__title">Authenticator app</h2>
        ${
          hasConfirmedTotp
            ? '<p class="gk-mfa__muted">An authenticator app is enrolled. Remove it above before registering a new one.</p>'
            : `
              <div class="gk-mfa__actions">
                <button class="gk-mfa__button gk-mfa__button--primary" type="button" data-action="start-totp" ${this.#disabledAttr()}>
                  Set up authenticator app
                </button>
              </div>
            `
        }
      </section>
    `;
  }

  #renderPasskeySection(): string {
    if (this.#addingPasskey) {
      return `
        <section class="gk-mfa__section" data-gatekeeper-mfa-section="passkeys">
          <h2 class="gk-mfa__title">Passkeys</h2>
          <form class="gk-mfa__actions" data-action="add-passkey">
            <input class="gk-mfa__input" name="passkey-name" type="text" maxlength="100" placeholder="Name (e.g. YubiKey 5C)" />
            <button class="gk-mfa__button gk-mfa__button--primary" type="submit" ${this.#disabledAttr()}>Create passkey</button>
            <button class="gk-mfa__button" type="button" data-action="cancel-passkey" ${this.#disabledAttr()}>Cancel</button>
          </form>
        </section>
      `;
    }

    return `
      <section class="gk-mfa__section" data-gatekeeper-mfa-section="passkeys">
        <h2 class="gk-mfa__title">Passkeys</h2>
        <p class="gk-mfa__muted">Register a passkey.</p>
        <div class="gk-mfa__actions">
          <button class="gk-mfa__button gk-mfa__button--primary" type="button" data-action="show-passkey-form" ${this.#disabledAttr()}>
            Add passkey
          </button>
        </div>
      </section>
    `;
  }

  #handleClick = (event: Event) => {
    const target = event.target as HTMLElement | null;
    const actionTarget = target?.closest<HTMLElement>('[data-action]');
    if (!actionTarget) return;

    const action = actionTarget.dataset.action;
    if (action === 'default') {
      const method = actionTarget.dataset.method as MfaMethod | undefined;
      if (method) void this.#run(() => this.client.setPreferredMethod(method));
    } else if (action === 'rename') {
      void this.#renameMethod(actionTarget.dataset.methodKey);
    } else if (action === 'remove') {
      void this.#removeMethod(actionTarget.dataset.methodKey);
    } else if (action === 'start-totp') {
      void this.#run(async () => {
        this.#totpEnrollment = await this.client.startTotpEnrollment();
      }, false);
    } else if (action === 'cancel-totp') {
      this.#totpEnrollment = null;
      this.render();
    } else if (action === 'copy-totp-secret') {
      void navigator.clipboard.writeText(this.#totpEnrollment?.secretBase32 ?? '');
    } else if (action === 'show-passkey-form') {
      this.#addingPasskey = true;
      this.render();
    } else if (action === 'cancel-passkey') {
      this.#addingPasskey = false;
      this.render();
    }
  };

  #handleSubmit = (event: Event) => {
    const form = event.target as HTMLFormElement | null;
    if (!form?.dataset.action) return;
    event.preventDefault();

    if (form.dataset.action === 'confirm-totp') {
      const input = form.elements.namedItem('totp-code') as HTMLInputElement | null;
      void this.#run(async () => {
        await this.client.confirmTotpEnrollment(input?.value ?? '');
        this.#totpEnrollment = null;
      });
    } else if (form.dataset.action === 'add-passkey') {
      const input = form.elements.namedItem('passkey-name') as HTMLInputElement | null;
      void this.#run(async () => {
        await this.client.addPasskey(input?.value ?? null);
        this.#addingPasskey = false;
      });
    }
  };

  async #run(action: () => Promise<void>, reload = true) {
    this.#errorMessage = '';
    this.#busy = true;
    this.render();

    try {
      await action();
      if (reload) await this.reload();
    } catch (error) {
      this.#setError(error);
    } finally {
      this.#busy = false;
      this.render();
    }
  }

  async #renameMethod(key: string | undefined) {
    const entry = this.#findMethod(key);
    if (!entry?.id) return;
    const name = prompt('New name for this method', entry.name ?? '');
    if (name === null) return;
    await this.#run(() => this.client.renameMethod(entry.id as string, name).then(() => undefined));
  }

  async #removeMethod(key: string | undefined) {
    const entry = this.#findMethod(key);
    if (!entry?.id) return;
    const approved = confirm(`Remove "${entry.name || defaultMfaMethodLabels[entry.method]}"? You will no longer be able to sign in with it.`);
    if (!approved) return;
    await this.#run(() => this.client.removeMethod(entry.id as string));
  }

  #findMethod(key: string | undefined): MfaMethodEntry | undefined {
    return this.#state.methods.find((entry) => methodKey(entry) === key);
  }

  #methodMeta(entry: MfaMethodEntry): string {
    if (!entry.confirmed) return 'Pending confirmation';
    if (entry.lastUsedAt) return `Last used ${new Date(entry.lastUsedAt).toLocaleString()}`;
    if (entry.createdAt) return `Added ${new Date(entry.createdAt).toLocaleDateString()}`;
    return 'Always available';
  }

  #setError(error: unknown) {
    this.#errorMessage = error instanceof Error ? error.message : String(error);
    this.render();
  }

  #emitChange() {
    this.dispatchEvent(
      new CustomEvent('gatekeeper:mfa-change', {
        bubbles: true,
        composed: true,
        detail: this.#state
      })
    );
  }

  #disabledAttr(): string {
    return this.#busy ? 'disabled' : '';
  }
}
