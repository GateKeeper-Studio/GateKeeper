<script>
  // Self-service MFA management. Every mutation goes through our /api/mfa/*
  // proxy routes (BFF): the browser never holds the IdP access token.
  import { StyledMfaConfiguration } from '$lib/components/gatekeeper';

  let { data } = $props();

  // ── Recent sign-in activity (read-only, loaded server-side) ─────────────────
  let loginEvents = $derived(data.loginEvents ?? []);

  function formatLocation(e) {
    const parts = [e.city, e.region, e.country_code].filter(Boolean);
    return parts.length ? parts.join(', ') : 'Unknown location';
  }

  function shortDevice(ua) {
    if (!ua) return 'Unknown device';
    // Light touch: surface the browser + OS without a full UA-parsing dependency.
    const m = ua.match(/(Firefox|Edg|OPR|Chrome|Safari)\/[\d.]+/);
    const browser = m ? m[1].replace('Edg', 'Edge').replace('OPR', 'Opera') : 'Browser';
    const os = /Windows/.test(ua)
      ? 'Windows'
      : /Mac OS X|Macintosh/.test(ua)
        ? 'macOS'
        : /Android/.test(ua)
          ? 'Android'
          : /iPhone|iPad|iOS/.test(ua)
            ? 'iOS'
            : /Linux/.test(ua)
              ? 'Linux'
              : '';
    return os ? `${browser} · ${os}` : browser;
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

  <StyledMfaConfiguration initialMfa={data.mfa} />

  <section class="card">
    <h2>Recent sign-in activity</h2>
    {#if loginEvents.length === 0}
      <p class="muted">No recent sign-in activity to show.</p>
    {:else}
      <ul class="method-list">
        {#each loginEvents as event (event.id)}
          <li class="method-row">
            <div class="method-info">
              <span class="method-name">{formatLocation(event)}</span>
              <span class="method-meta muted">
                {shortDevice(event.user_agent)}{event.ip_address ? ` · ${event.ip_address}` : ''} ·
                {new Date(event.created_at).toLocaleString()}
              </span>
            </div>
            {#if !event.success}
              <span class="muted">Failed</span>
            {:else if event.risk_level && event.risk_level.toLowerCase() !== 'low'}
              <span class="muted">{event.risk_level} risk</span>
            {/if}
          </li>
        {/each}
      </ul>
      <p class="muted">
        Don't recognise something here? Change your password and review your two-factor methods
        above.
      </p>
    {/if}
  </section>

  <div class="actions">
    <a class="button" href="/">Back to home</a>
    <a class="button" href="/account/security/headless-example">View headless MFA example</a>
  </div>
</main>
