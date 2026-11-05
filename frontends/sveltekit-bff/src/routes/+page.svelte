<script>
  // Home page. The whole OAuth dance happens server-side (BFF); this page only
  // reflects whether someone is signed in and offers Sign In / Sign Out.
  //
  // The MFA checkboxes are a per-browser *hint* for the next sign-in (which
  // methods the login UI offers). The authoritative default method lives
  // server-side and is managed on /account/security.
  import { page } from '$app/state';

  let { data } = $props();

  const mfaOptions = [
    { value: 'webauthn', label: 'Passkey' },
    { value: 'totp', label: 'Authenticator app' },
    { value: 'email', label: 'Email code' }
  ];

  const preferencesKey = 'gk_mfa_preferences';

  // Factors actually enrolled on the account (from /me/mfa/methods). Only these
  // can be hinted — a method you haven't set up can never be challenged, so we
  // disable its checkbox rather than letting it pretend to do something.
  let enrolledMethods = $derived(data.enrolledMfaMethods ?? []);

  // Which enrolled factors are checked for this browser's next sign-in. Starts
  // as "all enrolled"; refined from the saved preference once mounted.
  let enabledMfaMethods = $state(data.enrolledMfaMethods ?? []);
  let preferencesLoaded = $state(false);

  // Only enrolled *and* checked methods are valid hints, kept in display order.
  let selectedMfaMethods = $derived(
    mfaOptions
      .map((option) => option.value)
      .filter((method) => enrolledMethods.includes(method) && enabledMfaMethods.includes(method))
  );

  // Send the user back to wherever they were after signing in.
  let signInHref = $derived.by(() => {
    const params = new URLSearchParams({ returnTo: page.url.pathname });
    // Omit the hint when every enrolled factor is selected (login already offers
    // exactly those) or when none is — let the server-side default decide.
    if (selectedMfaMethods.length > 0 && selectedMfaMethods.length < enrolledMethods.length) {
      params.set('mfa_methods', selectedMfaMethods.join(','));
    }
    return `/auth/sign-in?${params.toString()}`;
  });

  $effect(() => {
    if (preferencesLoaded) return;
    const raw = localStorage.getItem(preferencesKey);
    if (raw) {
      try {
        const stored = JSON.parse(raw);
        const methods = Array.isArray(stored.methods)
          ? enrolledMethods.filter((method) => stored.methods.includes(method))
          : [];
        if (methods.length > 0) {
          enabledMfaMethods = methods;
        }
      } catch {
        localStorage.removeItem(preferencesKey);
      }
    }
    preferencesLoaded = true;
  });

  $effect(() => {
    if (!preferencesLoaded || !data.user) return;
    if (selectedMfaMethods.length === 0) return;

    localStorage.setItem(preferencesKey, JSON.stringify({ methods: selectedMfaMethods }));
  });

  function toggleMfaMethod(method, checked) {
    if (checked) {
      if (!enabledMfaMethods.includes(method)) {
        enabledMfaMethods = [...enabledMfaMethods, method];
      }
      return;
    }

    enabledMfaMethods = enabledMfaMethods.filter((value) => value !== method);
  }
</script>

<main>
  <header>
    <h1>OAuth 2.1 + OIDC Client <span class="muted">(Svelte BFF)</span></h1>
    <p class="subtitle">
      Authorization Code + PKCE against the Rust Identity Provider. Tokens are exchanged and
      stored <strong>server-side</strong> in an encrypted, httpOnly cookie — never exposed to the
      browser.
    </p>
  </header>

  <section class="card">
    {#if data.user}
      <h2>Signed in</h2>
      <dl class="kv">
        {#if data.user.name}<dt>Name</dt><dd>{data.user.name}</dd>{/if}
        {#if data.user.email}<dt>Email</dt><dd>{data.user.email}</dd>{/if}
        <dt>Subject</dt><dd>{data.user.sub}</dd>
      </dl>

      <div class="mfa-config">
        <div>
          <h3>MFA sign-in hints</h3>
          <p class="muted">
            Methods offered on this browser's next sign-in. Enroll factors and pick your default
            method in <a href="/account/security">Security settings</a>.
          </p>
        </div>

        <div class="mfa-methods" aria-label="Allowed MFA methods">
          {#each mfaOptions as option}
            {@const enrolled = enrolledMethods.includes(option.value)}
            <label class="check-row" class:disabled={!enrolled}>
              <input
                type="checkbox"
                checked={enrolled && enabledMfaMethods.includes(option.value)}
                disabled={!enrolled}
                onchange={(event) => toggleMfaMethod(option.value, event.currentTarget.checked)}
              />
              <span>{option.label}</span>
              {#if !enrolled}
                <span class="not-set-up muted">
                  Not set up · <a href="/account/security">Set up</a>
                </span>
              {/if}
            </label>
          {/each}
        </div>

        {#if enrolledMethods.length === 0}
          <p class="muted">
            No second factor is enrolled yet. <a href="/account/security">Set one up</a> to choose
            sign-in hints.
          </p>
        {:else if selectedMfaMethods.length === 0}
          <p class="form-error">Select at least one method for your next sign-in.</p>
        {/if}
      </div>

      <div class="actions">
        <a class="button primary" href="/account/security">Security settings</a>
        <a class="button" href="/protected">Go to protected page</a>
        <form method="POST" action="/auth/sign-out">
          <button class="danger" type="submit">Sign out</button>
        </form>
      </div>
    {:else}
      <h2>You are not signed in</h2>
      <p class="muted">MFA methods can be managed after sign-in under Security settings.</p>
      <div class="actions">
        <a class="button primary" href={signInHref}>Sign In</a>
        <a class="button" href="/protected">Try a protected page</a>
      </div>
    {/if}
  </section>
</main>
