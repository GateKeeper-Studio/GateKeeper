<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';

	import {
		exportUserData,
		eraseUser,
		listUserAcceptances
	} from '$lib/services/admin/compliance';

	let { tenantId, userId }: { tenantId: string; userId: string } = $props();

	let exporting = $state(false);
	let erasing = $state(false);
	let actionError = $state<string | null>(null);
	let erased = $state(false);

	const acceptancesQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId, 'acceptances'],
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await listUserAcceptances(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));

	async function handleExport() {
		actionError = null;
		exporting = true;
		const [data, err] = await exportUserData(tenantId, userId);
		exporting = false;
		if (err) {
			actionError = err.message;
			return;
		}
		// Trigger a client-side JSON download (GDPR Art. 20 portability).
		const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `data-export-${userId}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	async function handleErase() {
		if (
			!confirm(
				'Erase this user? Their personal data will be anonymized and they will be unable to sign in. This cannot be undone.'
			)
		) {
			return;
		}
		actionError = null;
		erasing = true;
		const [, err] = await eraseUser(tenantId, userId);
		erasing = false;
		if (err) {
			actionError = err.message;
			return;
		}
		erased = true;
	}
</script>

<div class="space-y-6">
	{#if actionError}
		<ErrorAlert title="Action failed" message={actionError} />
	{/if}

	<div class="rounded-xl border p-4">
		<h3 class="text-lg font-semibold">Data subject rights</h3>
		<p class="text-muted-foreground mt-1 text-sm">
			Export this user's personal data (right of access / portability) or erase it (right to be
			forgotten). Both actions are recorded in the audit log.
		</p>
		<div class="mt-4 flex flex-wrap gap-3">
			<Button variant="outline" onclick={handleExport} disabled={exporting || erased}>
				{exporting ? 'Exporting…' : 'Export data (JSON)'}
			</Button>
			<Button variant="destructive" onclick={handleErase} disabled={erasing || erased}>
				{erasing ? 'Erasing…' : erased ? 'Erased' : 'Erase / anonymize'}
			</Button>
		</div>
		{#if erased}
			<p class="mt-3 text-sm text-green-600">
				User anonymized. Their personal data has been scrubbed.
			</p>
		{/if}
	</div>

	<div class="rounded-xl border p-4">
		<h3 class="text-lg font-semibold">Policy acceptances</h3>
		<p class="text-muted-foreground mt-1 text-sm">
			Versions of legal documents this user has accepted.
		</p>
		<div class="mt-4">
			{#if acceptancesQuery.isPending}
				<Skeleton class="h-20 w-full rounded-lg" />
			{:else if acceptancesQuery.isError}
				<ErrorAlert title="Couldn't load acceptances" message={acceptancesQuery.error.message} />
			{:else if acceptancesQuery.data.length === 0}
				<p class="text-muted-foreground text-sm">No policy acceptances recorded.</p>
			{:else}
				<ul class="divide-y">
					{#each acceptancesQuery.data as a (a.document_type + a.document_version)}
						<li class="flex items-center justify-between py-2 text-sm">
							<span class="font-medium">{a.document_type}</span>
							<span class="text-muted-foreground">
								v{a.document_version} · {new Date(a.accepted_at).toLocaleDateString()}
							</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</div>
