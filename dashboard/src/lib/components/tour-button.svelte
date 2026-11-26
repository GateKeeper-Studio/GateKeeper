<script lang="ts">
	import CircleHelpIcon from '@lucide/svelte/icons/circle-help';

	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button/index.js';
	import { startConsoleTour } from '$lib/tour/console-tour';
	import { m } from '$lib/paraglide/messages.js';

	// Replays the guided tour on any console page: the console shell, plus the current
	// page's own steps when it has some.
	function replay() {
		void startConsoleTour({ admin: page.data.admin, routeId: page.route.id });
	}
</script>

<Button
	variant="outline"
	size="icon"
	onclick={replay}
	aria-label={m.tour_button_label()}
	title={m.tour_button_label()}
	data-tour="tour-button"
>
	<CircleHelpIcon />
</Button>
