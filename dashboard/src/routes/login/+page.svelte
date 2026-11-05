<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { z } from 'zod';

	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import MailIcon from '@lucide/svelte/icons/mail';
	import LockIcon from '@lucide/svelte/icons/lock';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import CpuIcon from '@lucide/svelte/icons/cpu';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import FingerprintIcon from '@lucide/svelte/icons/fingerprint';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';

	import { validate } from '$lib/utils/form';
	import { adminLogin } from '$lib/services/auth/admin-login';
	import { websiteHref } from '$lib/external-urls';
	import { m } from '$lib/paraglide/messages.js';

	const schema = z.object({
		email: z.email(m.admin_login_invalid_email()),
		password: z.string().min(1, m.admin_login_password_required())
	});

	let email = $state('');
	let password = $state('');
	let showPassword = $state(false);
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	const highlights = [
		{ icon: CpuIcon, text: m.admin_login_highlight_rust() },
		{ icon: KeyRoundIcon, text: m.admin_login_highlight_oauth() },
		{ icon: FingerprintIcon, text: m.admin_login_highlight_mfa() }
	];

	function showError(message: string) {
		error = message;
		isLoading = false;
		setTimeout(() => (error = null), 6000);
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(schema, { email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [data, err] = await adminLogin(result.data.email.trim(), result.data.password);

		if (err || !data) {
			showError(err?.data?.error || err?.data?.message || m.admin_login_invalid_credentials());
			return;
		}

		// Nothing to store: the session is an httpOnly cookie the server just set.
		// `next` is the path the guard bounced us from — it is always a path+query
		// from `+layout.server.ts`, never an absolute URL, so it cannot redirect
		// off-site; re-check that here rather than trusting the query string.
		const next = page.url.searchParams.get('next');
		const target = next && next.startsWith('/') && !next.startsWith('//') ? next : resolve('/');
		await goto(target);
	}
</script>

<svelte:head>
	<title>{m.admin_login_meta_title()} - GateKeeper</title>
	<meta name="description" content={m.admin_login_meta_description()} />
</svelte:head>

<div class="grid min-h-screen lg:grid-cols-2">
	<!-- Form column -->
	<div class="relative flex flex-col px-4 py-6 sm:px-8">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2"><LanguageSwitcher /><ThemeToggle /></div>
		</div>

		<div class="flex flex-1 items-center justify-center">
			<div class="w-full max-w-sm animate-in py-10 duration-700 fade-in slide-in-from-bottom-4">
				<!-- Wordmark (shown here on mobile; brand panel carries it on desktop) -->
				<a
					href={websiteHref('/')}
					class="mb-8 inline-flex items-center gap-2 font-semibold tracking-tight lg:hidden"
				>
					<span
						class="flex size-8 items-center justify-center rounded-lg bg-gradient-to-br from-brand to-brand-2 text-brand-foreground"
					>
						<ShieldCheckIcon class="size-5" />
					</span>
					<span class="text-lg">GateKeeper</span>
				</a>

				<h1 class="text-2xl font-semibold tracking-tight">{m.admin_login_title()}</h1>
				<p class="mt-1.5 text-sm text-muted-foreground">
					{m.admin_login_description()}
				</p>

				{#if error}
					<div class="mt-6">
						<ErrorAlert message={error} title={m.auth_error_title()} />
					</div>
				{/if}

				<form onsubmit={onSubmit} class="mt-6 space-y-4">
					<div class="space-y-2">
						<Label for="email">{m.auth_email_label()}</Label>
						<div class="relative">
							<MailIcon
								class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
							/>
							<Input
								id="email"
								type="email"
								autocomplete="email"
								placeholder="admin@example.com"
								class="pl-9"
								aria-invalid={errors.email ? 'true' : undefined}
								bind:value={email}
							/>
						</div>
						{#if errors.email}
							<p class="text-sm font-medium text-destructive">{errors.email}</p>
						{/if}
					</div>

					<div class="space-y-2">
						<Label for="password">{m.auth_password_label()}</Label>
						<div class="relative">
							<LockIcon
								class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
							/>
							<Input
								id="password"
								type={showPassword ? 'text' : 'password'}
								autocomplete="current-password"
								placeholder="••••••••"
								class="px-9"
								aria-invalid={errors.password ? 'true' : undefined}
								bind:value={password}
							/>
							<button
								type="button"
								onclick={() => (showPassword = !showPassword)}
								class="absolute top-1/2 right-2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground transition-colors hover:text-foreground"
								aria-label={showPassword
									? m.admin_login_hide_password()
									: m.admin_login_show_password()}
							>
								{#if showPassword}
									<EyeOffIcon class="size-4" />
								{:else}
									<EyeIcon class="size-4" />
								{/if}
							</button>
						</div>
						{#if errors.password}
							<p class="text-sm font-medium text-destructive">{errors.password}</p>
						{/if}
					</div>

					<Button
						type="submit"
						disabled={isLoading}
						class="relative w-full bg-gradient-to-r from-brand to-brand-2 text-brand-foreground shadow-sm transition-opacity hover:opacity-90"
					>
						{#if isLoading}
							<LoadingSpinner class="absolute left-4" />
						{/if}
						{m.admin_login_submit()}
					</Button>
				</form>

				<p class="mt-8 text-center text-xs text-muted-foreground">
					{m.admin_login_legal_prefix()}
					<a href={resolve('/terms')} class="underline underline-offset-4 hover:text-foreground"
						>{m.landing_footer_link_terms()}</a
					>
					{m.admin_login_legal_and()}
					<a href={resolve('/privacy')} class="underline underline-offset-4 hover:text-foreground"
						>{m.landing_footer_link_privacy()}</a
					>.
				</p>
			</div>
		</div>
	</div>

	<!-- Brand panel (desktop only) -->
	<div
		class="relative hidden overflow-hidden bg-gradient-to-br from-brand to-brand-2 lg:flex lg:flex-col lg:justify-between lg:p-12"
	>
		<!-- decorative aurora + grid -->
		<div class="pointer-events-none absolute inset-0" aria-hidden="true">
			<div
				class="gk-aurora absolute -top-24 -right-16 size-96 rounded-full bg-white/20 blur-3xl"
			></div>
			<div
				class="gk-aurora absolute -bottom-24 -left-16 size-96 rounded-full bg-white/10 blur-3xl [animation-delay:-6s]"
			></div>
			<div
				class="absolute inset-0 [background-image:linear-gradient(to_right,white_1px,transparent_1px),linear-gradient(to_bottom,white_1px,transparent_1px)] [mask-image:radial-gradient(ellipse_at_center,black,transparent_75%)] [background-size:48px_48px] opacity-10"
			></div>
		</div>

		<a
			href={websiteHref('/')}
			class="relative flex items-center gap-2 font-semibold tracking-tight text-white"
		>
			<span class="flex size-9 items-center justify-center rounded-lg bg-white/15 backdrop-blur">
				<ShieldCheckIcon class="size-5" />
			</span>
			<span class="text-lg">GateKeeper</span>
		</a>

		<div class="relative max-w-md">
			<h2 class="text-3xl font-semibold tracking-tight text-balance text-white">
				{m.admin_login_panel_title()}
			</h2>
			<p class="mt-4 text-pretty text-white/80">
				{m.admin_login_panel_description()}
			</p>

			<ul class="mt-8 space-y-3">
				{#each highlights as item (item.text)}
					{@const Icon = item.icon}
					<li class="flex items-center gap-3 text-white">
						<span
							class="flex size-8 items-center justify-center rounded-lg bg-white/15 backdrop-blur"
						>
							<Icon class="size-4" />
						</span>
						<span class="text-sm font-medium">{item.text}</span>
					</li>
				{/each}
			</ul>
		</div>

		<p class="relative text-sm text-white/60">
			{m.landing_footer_copyright({ year: new Date().getFullYear() })}
		</p>
	</div>
</div>
