<script lang="ts">
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';
	import TerminalIcon from '@lucide/svelte/icons/terminal';
	import ZapIcon from '@lucide/svelte/icons/zap';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import PlugIcon from '@lucide/svelte/icons/plug';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { reveal } from '$lib/actions/reveal';
	import { m } from '$lib/paraglide/messages.js';

	type Snippet = { id: string; label: string; file: string; code: string };

	const snippets: Snippet[] = $derived([
		{
			id: 'react',
			label: m.landing_code_sample_tab_react(),
			file: 'app.tsx',
			code: `import { GateKeeperProvider, useAuth } from '@gatekeeper/react';

// Wrap your app once
export function App() {
  return (
    <GateKeeperProvider domain='your-tenant.gatekeeper.dev' clientId='spa_x1a2'>
      <Dashboard />
    </GateKeeperProvider>
  );
}

// Then guard anything with the hook
function Dashboard() {
  const { user, login, logout } = useAuth();
  if (!user) return <button onClick={login}>Sign in</button>;
  return <p>Welcome back, {user.name}</p>;
}`
		},
		{
			id: 'node',
			label: m.landing_code_sample_tab_node(),
			file: 'server.ts',
			code: `import { createGateKeeper } from '@gatekeeper/bff-core';

const auth = createGateKeeper({
  domain: 'your-tenant.gatekeeper.dev',
  clientId: 'bff_9f8e',
  clientSecret: process.env.GK_SECRET,
  redirectUri: 'https://app.example.com/callback'
});

// Exchange the code server-side — tokens never touch the browser
app.get('/callback', async (req, res) => {
  const session = await auth.handleCallback(req.query.code);
  res.cookie('sid', session.id, { httpOnly: true, secure: true });
  res.redirect('/');
});`
		}
	]);

	let active = $state(snippets[0].id);
	let copied = $state(false);

	const current = $derived(snippets.find((s) => s.id === active) ?? snippets[0]);

	async function copy() {
		try {
			await navigator.clipboard.writeText(current.code);
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			// clipboard unavailable — no-op
		}
	}

	// Lightweight syntax highlighting for known, static snippets.
	function esc(s: string) {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}
	function highlight(code: string) {
		const re =
			/('[^']*')|(\/\/[^\n]*)|\b(import|from|export|const|function|return|await|new|async|if)\b/g;
		return esc(code).replace(re, (m, str, comment, kw) => {
			if (str) return `<span class="text-brand-2">${str}</span>`;
			if (comment) return `<span class="text-muted-foreground italic">${comment}</span>`;
			if (kw) return `<span class="text-brand font-medium">${kw}</span>`;
			return m;
		});
	}

	const perks = $derived([
		{ icon: ZapIcon, text: m.landing_code_sample_perk_fast() },
		{ icon: ShieldCheckIcon, text: m.landing_code_sample_perk_secure() },
		{ icon: PlugIcon, text: m.landing_code_sample_perk_adapters() }
	]);
</script>

<!-- highlight() HTML-escapes its input and only wraps static, trusted snippets,
     so {@html} below is safe. -->
<!-- eslint-disable svelte/no-at-html-tags -->

<section id="developers" class="scroll-mt-20 border-y border-border/60 bg-muted/30">
	<div class="mx-auto grid max-w-7xl gap-12 px-4 py-24 sm:px-6 lg:grid-cols-2 lg:items-center">
		<div use:reveal>
			<span class="text-sm font-semibold tracking-wide text-brand uppercase"
				>{m.landing_code_sample_eyebrow()}</span
			>
			<h2 class="mt-3 text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
				{m.landing_code_sample_heading()}
			</h2>
			<p class="mt-4 text-lg text-pretty text-muted-foreground">
				{m.landing_code_sample_subheading()}
			</p>

			<ul class="mt-8 space-y-3">
				{#each perks as perk (perk.text)}
					{@const Icon = perk.icon}
					<li class="flex items-center gap-3">
						<span
							class="flex size-8 items-center justify-center rounded-lg border border-border bg-background text-brand"
						>
							<Icon class="size-4" />
						</span>
						<span class="font-medium">{perk.text}</span>
					</li>
				{/each}
			</ul>

			<Button href="/docs" variant="outline" class="mt-8">
				<TerminalIcon class="size-4" />
				{m.landing_code_sample_docs_cta()}
			</Button>
		</div>

		<div use:reveal={{ delay: 120 }}>
			<div class="overflow-hidden rounded-2xl border border-border bg-card shadow-xl">
				<Tabs.Root bind:value={active}>
					<div class="flex items-center justify-between border-b border-border bg-muted/50 pr-2">
						<Tabs.List class="bg-transparent">
							{#each snippets as snippet (snippet.id)}
								<Tabs.Trigger value={snippet.id}>{snippet.label}</Tabs.Trigger>
							{/each}
						</Tabs.List>
						<div class="flex items-center gap-2">
							<span class="hidden text-xs text-muted-foreground sm:inline">{current.file}</span>
							<Button
								variant="ghost"
								size="icon-sm"
								onclick={copy}
								aria-label={m.landing_code_sample_copy_aria_label()}
							>
								{#if copied}
									<CheckIcon class="size-4 text-brand-2" />
								{:else}
									<CopyIcon class="size-4" />
								{/if}
							</Button>
						</div>
					</div>

					{#each snippets as snippet (snippet.id)}
						<Tabs.Content value={snippet.id} class="mt-0">
							<pre class="overflow-x-auto p-5 text-sm leading-relaxed"><code class="font-mono"
									>{@html highlight(snippet.code)}</code
								></pre>
						</Tabs.Content>
					{/each}
				</Tabs.Root>
			</div>
		</div>
	</div>
</section>
