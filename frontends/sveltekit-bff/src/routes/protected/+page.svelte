<script>
  // Only reachable when signed in (see +page.server.js).
  let { data } = $props();
</script>

<main>
  <header>
    <h1>Protected page</h1>
    <p class="subtitle">You only see this because you have a valid server-side session.</p>
  </header>

  <section class="card">
    <h2>Hello{data.user?.name ? `, ${data.user.name}` : ''}</h2>
    <dl class="kv">
      {#if data.user?.email}<dt>Email</dt><dd>{data.user.email}</dd>{/if}
      <dt>Subject</dt><dd>{data.user?.sub}</dd>
    </dl>

    <div class="actions">
      <a class="button" href="/">Home</a>
      <form method="POST" action="/auth/sign-out">
        <button class="danger" type="submit">Sign out</button>
      </form>
    </div>
  </section>
</main>
