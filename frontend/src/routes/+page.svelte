<script>
  // Orchestrates the OAuth 2.1 Authorization Code + PKCE flow and the small UI.
  //
  // Threat-model notes baked in here:
  //  - Transient secrets (code_verifier, state, nonce) live in sessionStorage and
  //    are deleted the moment they're consumed.
  //  - Tokens are held in component state (memory) only — never localStorage —
  //    so they vanish on tab close and aren't readable by a later page load.
  //  - state is verified on return (CSRF); nonce is verified against the id_token.
  //  - All server-provided strings are rendered as text by Svelte (auto-escaped),
  //    never as HTML.

  import { onMount } from 'svelte';
  import { loadConfig, saveConfig, defaultConfig, issuerBase } from '$lib/config.js';
  import { generatePkce } from '$lib/pkce.js';
  import { decodeJwt, verifyRs256, validateIdTokenClaims } from '$lib/jwt.js';
  import * as api from '$lib/api.js';

  const SS_PKCE = 'oauth_pkce'; // sessionStorage key for the in-flight PKCE/state/nonce

  let cfg = $state({
    issuer: '',
    tenantId: '',
    clientId: '',
    clientSecret: '',
    redirectUri: '',
    scope: '',
    userId: ''
  });

  let validated = $state(null); // result of GET /oauth/authorize
  let tokens = $state(null); // { access_token, id_token, refresh_token, ... } — memory only
  let logs = $state([]); // [{ time, message, kind }]

  let discovery = $state(null);
  let userinfo = $state(null);

  let consentVisible = $state(false);
  let tokensVisible = $state(false);

  // Decoded views, filled imperatively because JWKS verification is async.
  let accessView = $state(null); // { token, claims }
  let idView = $state(null); // { token, claims, checks } | { message }

  onMount(() => {
    cfg = loadConfig();
    log('Ready. Fill in config and click “Start login”.');
  });

  // ── logging ──────────────────────────────────────────────────────────────
  function log(message, kind = 'info') {
    const time = new Date().toLocaleTimeString();
    logs = [{ time, message, kind }, ...logs];
  }

  function logError(prefix, err) {
    log(`${prefix}: ${err.message || err}`, 'error');
  }

  // ── config ─────────────────────────────────────────────────────────────────
  function configValid() {
    const required = ['issuer', 'tenantId', 'clientId', 'redirectUri', 'scope', 'userId'];
    const missing = required.filter((f) => !cfg[f]?.trim());
    if (missing.length) {
      log(`Missing required config: ${missing.join(', ')}`, 'error');
      return false;
    }
    return true;
  }

  // ── flow ─────────────────────────────────────────────────────────────────────
  async function startFlow() {
    for (const k of Object.keys(cfg)) cfg[k] = (cfg[k] || '').trim();
    saveConfig(cfg);
    if (!configValid()) return;

    try {
      const pkce = await generatePkce();
      // Persist the transient values so a real redirect round-trip could resume.
      sessionStorage.setItem(SS_PKCE, JSON.stringify(pkce));
      log('Generated PKCE verifier/challenge (S256), state, and nonce.');

      validated = await api.authorizeRequest(cfg, pkce);
      log('Authorization request validated by the provider.', 'ok');

      consentVisible = true;
    } catch (err) {
      logError('Authorize', err);
    }
  }

  async function approveConsent() {
    consentVisible = false;
    const pkce = JSON.parse(sessionStorage.getItem(SS_PKCE) || 'null');
    if (!pkce) {
      log('No in-flight request — start the flow again.', 'error');
      return;
    }

    try {
      const { code, state: returnedState } = await api.consent(cfg, pkce, validated);
      log('Received single-use authorization code.', 'ok');

      // CSRF defense: the state returned with the code must match what we sent.
      if (returnedState !== pkce.state) {
        throw new Error('state mismatch — possible CSRF, aborting');
      }
      log('state parameter verified.', 'ok');

      const t = await api.exchangeCode(cfg, code, pkce.codeVerifier);
      tokens = t;
      log('Exchanged code for tokens.', 'ok');

      // The verifier/state/nonce have served their purpose — destroy them.
      sessionStorage.removeItem(SS_PKCE);

      await showTokens(t, pkce.nonce);
      tokensVisible = true;
    } catch (err) {
      logError('Token exchange', err);
    }
  }

  function denyConsent() {
    consentVisible = false;
    sessionStorage.removeItem(SS_PKCE);
    log('Consent denied by user.', 'warn');
  }

  async function showTokens(t, nonce) {
    // Decode + display the access token (it's a JWT in this provider).
    try {
      const { payload } = decodeJwt(t.access_token);
      accessView = { token: t.access_token, claims: payload };
    } catch {
      accessView = { token: t.access_token, claims: null };
    }

    // Decode + cryptographically verify the id_token against JWKS.
    if (!t.id_token) {
      idView = { message: "No id_token (request the 'openid' scope to receive one)." };
      return;
    }

    const { payload } = decodeJwt(t.id_token);
    const checks = validateIdTokenClaims(payload, {
      issuer: issuerBase(cfg),
      clientId: cfg.clientId,
      nonce
    });

    try {
      const keys = await api.jwks(cfg);
      const verified = await verifyRs256(t.id_token, keys);
      checks.unshift({ label: 'RS256 signature verified against JWKS', ok: verified });
    } catch (err) {
      checks.unshift({ label: `signature verification skipped (${err.message})`, ok: false });
    }

    idView = { token: t.id_token, claims: payload, checks };
    log('ID token decoded and validated.', checks.every((c) => c.ok) ? 'ok' : 'warn');
  }

  async function callUserInfo() {
    if (!tokens?.access_token) {
      log('No access token — complete the flow first.', 'error');
      return;
    }
    try {
      userinfo = await api.userInfo(cfg, tokens.access_token);
      log('UserInfo retrieved.', 'ok');
    } catch (err) {
      logError('UserInfo', err);
    }
  }

  async function doRefresh() {
    if (!tokens?.refresh_token) {
      log('No refresh token available.', 'error');
      return;
    }
    try {
      const t = await api.refresh(cfg, tokens.refresh_token);
      tokens = t;
      log('Refresh token rotated; new tokens issued.', 'ok');
      await showTokens(t, null);
    } catch (err) {
      logError('Refresh', err);
    }
  }

  async function showDiscovery() {
    try {
      discovery = await api.discovery(cfg);
      log('Fetched discovery document.', 'ok');
    } catch (err) {
      logError('Discovery', err);
    }
  }

  function logout() {
    tokens = null;
    validated = null;
    sessionStorage.removeItem(SS_PKCE);
    accessView = null;
    idView = null;
    userinfo = null;
    tokensVisible = false;
    consentVisible = false;
    log('Cleared all tokens and session state.', 'warn');
  }

  function resetConfig() {
    cfg = defaultConfig();
    log('Config reset to defaults.');
  }

  // Normalizes an object into [key, displayString] pairs for the <dl> renderer.
  function kvEntries(obj) {
    return Object.entries(obj ?? {}).map(([key, value]) => [
      key,
      typeof value === 'string' ? value : JSON.stringify(value)
    ]);
  }

  let tokenSummary = $derived(
    tokens
      ? {
          token_type: tokens.token_type,
          expires_in: tokens.expires_in,
          scope: tokens.scope,
          has_id_token: Boolean(tokens.id_token),
          has_refresh_token: Boolean(tokens.refresh_token)
        }
      : {}
  );
</script>

<main>
  <header>
    <h1>OAuth 2.1 + OIDC Test Client <span class="muted">(Svelte)</span></h1>
    <p class="subtitle">
      Authorization Code flow with mandatory PKCE (S256), against the Rust provider. This is a
      <strong>public client</strong>: PKCE only, tokens kept in memory.
    </p>
  </header>

  <section class="card">
    <h2>1 · Configuration</h2>
    <div class="grid">
      <label>
        Issuer (provider base URL)
        <input type="url" autocomplete="off" placeholder="http://localhost:8080" bind:value={cfg.issuer} />
      </label>
      <label>
        Tenant ID
        <input type="text" autocomplete="off" placeholder="UUID" bind:value={cfg.tenantId} />
      </label>
      <label>
        Client ID
        <input type="text" autocomplete="off" placeholder="registered client_id" bind:value={cfg.clientId} />
      </label>
      <label>
        Client secret <span class="muted">(leave empty for public clients)</span>
        <input type="password" autocomplete="off" bind:value={cfg.clientSecret} />
      </label>
      <label>
        Redirect URI <span class="muted">(must match registration)</span>
        <input type="url" autocomplete="off" bind:value={cfg.redirectUri} />
      </label>
      <label>
        Scope
        <input type="text" autocomplete="off" placeholder="openid profile email" bind:value={cfg.scope} />
      </label>
      <label>
        User ID <span class="muted">(/oauth/login not wired yet — pass the subject directly)</span>
        <input type="text" autocomplete="off" placeholder="user UUID" bind:value={cfg.userId} />
      </label>
    </div>
    <div class="actions">
      <button class="primary" type="button" onclick={startFlow}>Start login</button>
      <button type="button" onclick={showDiscovery}>Fetch discovery</button>
      <button type="button" onclick={resetConfig}>Reset config</button>
    </div>
  </section>

  {#if discovery}
    <section class="card">
      <h2>Discovery document</h2>
      <div class="kv">
        <dl>
          {#each kvEntries(discovery) as [k, v]}
            <dt>{k}</dt>
            <dd>{v}</dd>
          {/each}
        </dl>
      </div>
    </section>
  {/if}

  {#if consentVisible}
    <section class="card consent">
      <h2>2 · Consent</h2>
      <p><strong>{validated?.client_name || cfg.clientId}</strong> is requesting access to:</p>
      <ul class="scopes">
        {#each validated?.scopes ?? [] as scope}
          <li>{scope}</li>
        {/each}
      </ul>
      <div class="actions">
        <button class="primary" type="button" onclick={approveConsent}>Allow</button>
        <button class="danger" type="button" onclick={denyConsent}>Deny</button>
      </div>
    </section>
  {/if}

  {#if tokensVisible}
    <section class="card">
      <h2>3 · Tokens</h2>
      <div class="kv">
        <dl>
          {#each kvEntries(tokenSummary) as [k, v]}
            <dt>{k}</dt>
            <dd>{v}</dd>
          {/each}
        </dl>
      </div>

      <div class="actions">
        <button type="button" onclick={callUserInfo}>Call /userinfo</button>
        <button type="button" onclick={doRefresh}>Refresh tokens</button>
        <button class="danger" type="button" onclick={logout}>Log out / clear</button>
      </div>

      {#if idView}
        <div class="token-view">
          {#if idView.message}
            <p>{idView.message}</p>
          {:else}
            <h4>ID token (decoded)</h4>
            <pre class="token">{idView.token}</pre>
            <pre class="claims">{JSON.stringify(idView.claims, null, 2)}</pre>
            <ul class="checks">
              {#each idView.checks as c}
                <li class={c.ok ? 'ok' : 'bad'}>{c.ok ? '✓' : '✗'} {c.label}</li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      {#if accessView}
        <div class="token-view">
          <h4>Access token{accessView.claims ? ' (decoded)' : ''}</h4>
          <pre class="token">{accessView.token}</pre>
          {#if accessView.claims}
            <pre class="claims">{JSON.stringify(accessView.claims, null, 2)}</pre>
          {/if}
        </div>
      {/if}

      <h3>UserInfo</h3>
      <div class="kv">
        {#if userinfo}
          <dl>
            {#each kvEntries(userinfo) as [k, v]}
              <dt>{k}</dt>
              <dd>{v}</dd>
            {/each}
          </dl>
        {/if}
      </div>
    </section>
  {/if}

  <section class="card">
    <div class="log-header">
      <h2>Log</h2>
      <button type="button" onclick={() => (logs = [])}>Clear</button>
    </div>
    <div class="log" aria-live="polite">
      {#each logs as line}
        <div class="log-line log-{line.kind}">[{line.time}] {line.message}</div>
      {/each}
    </div>
  </section>
</main>
