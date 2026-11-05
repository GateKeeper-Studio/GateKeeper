<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import {
		DEFAULT_RESOLVED_LOGIN_APPEARANCE,
		fontStackFor,
		type LoginAppearance,
		type ResolvedLoginAppearance
	} from '$lib/types/login-appearance';
	import Centered from './layouts/centered.svelte';
	import SplitImage from './layouts/split-image.svelte';
	import Card from './layouts/card.svelte';
	import Minimal from './layouts/minimal.svelte';

	let {
		client,
		termsAndConditionsEnabled = true,
		children
	}: {
		client: ClientAuthData | null;
		termsAndConditionsEnabled?: boolean;
		children: Snippet;
	} = $props();

	// Live-preview mode: the admin dashboard embeds this page in a sandboxed iframe
	// (?preview=1) and postMessages a draft appearance as the admin edits. We only
	// listen when explicitly in preview mode, and only accept same-origin messages.
	const preview = $derived(page.url.searchParams.get('preview') === '1');
	// undefined = nothing received yet (show the client's saved look); null = the
	// admin toggled customization OFF (default look); object = a draft to render.
	let previewAppearance = $state<ResolvedLoginAppearance | null | undefined>(undefined);

	$effect(() => {
		if (!preview || typeof window === 'undefined') return;

		function onMessage(event: MessageEvent) {
			if (event.origin !== window.location.origin) return;
			const data = event.data;
			if (data && data.type === 'gk-preview-appearance') {
				// The admin's draft is still map-based (per-language heading/subtext) —
				// resolve it to this page's own current locale before rendering, same
				// fallback chain the server uses for the real (non-preview) response.
				const draft = (data.appearance as LoginAppearance | null) ?? null;
				const loc = getLocale();
				const fallback = client?.defaultLanguage ?? 'en';
				previewAppearance = draft
					? {
							...draft,
							heading: resolveLocalized(draft.heading, loc, fallback) || null,
							subtext: resolveLocalized(draft.subtext, loc, fallback) || null
						}
					: null;
			}
		}

		window.addEventListener('message', onMessage);
		// Tell the embedder we're ready so it can send the current draft immediately.
		window.parent?.postMessage({ type: 'gk-preview-ready' }, window.location.origin);
		return () => window.removeEventListener('message', onMessage);
	});

	// The appearance actually rendered: a live preview draft wins in preview mode,
	// otherwise the client's saved appearance. Merged over the defaults so every
	// field is always present.
	const effective = $derived(
		preview && previewAppearance !== undefined
			? previewAppearance
			: (client?.loginAppearance ?? null)
	);
	const appearance = $derived<ResolvedLoginAppearance>({
		...DEFAULT_RESOLVED_LOGIN_APPEARANCE,
		...(effective ?? {})
	});

	const layouts = { centered: Centered, split_image: SplitImage, card: Card, minimal: Minimal };
	const LayoutComponent = $derived(layouts[appearance.layout] ?? SplitImage);

	// Empty-string guards keep `style:` from emitting a blank custom property.
	const fontFamily = $derived(fontStackFor(appearance.fontFamily));
</script>

<!--
  Theme tokens are applied here via `style:` directives — Svelte sets each via the
  CSSOM (element.style.setProperty), never string concatenation. Every value was
  validated server-side (hex color, bounded rem/px radius, whitelisted font key),
  so there is no way to break out of the CSS context. `display:contents` means this
  wrapper generates no box but still cascades the inherited custom properties and
  font-family into the chosen layout.
-->
<div
	class="contents"
	style:--primary={appearance.primaryColor || null}
	style:--accent={appearance.accentColor || null}
	style:--radius={appearance.borderRadius || null}
	style:font-family={fontFamily || null}
>
	<LayoutComponent {client} {appearance} {termsAndConditionsEnabled}>
		{@render children()}
	</LayoutComponent>
</div>
