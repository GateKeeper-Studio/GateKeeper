<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getRiskPolicy, updateRiskPolicy, type RiskPolicyView } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const queryClient = useQueryClient();

	type Form = Omit<RiskPolicyView, 'tenant_id'>;
	let form = $state<Form | null>(null);
	let saving = $state(false);

	const policyQuery = createQuery(() => ({
		queryKey: ['risk-policy', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await getRiskPolicy(tenantId);
			if (err) throw err;
			return data;
		}
	}));

	// Seed the editable form once the policy loads (or defaults come back).
	$effect(() => {
		if (policyQuery.data && form === null) {
			const { tenant_id: _t, ...rest } = policyQuery.data;
			form = { ...rest };
		}
	});

	function validate(f: Form): string | null {
		if (f.medium_risk_threshold >= f.high_risk_threshold) return m.security_risk_policy_validation_medium_below_high();
		if (f.high_risk_threshold >= f.critical_risk_threshold) return m.security_risk_policy_validation_high_below_critical();
		if (f.failed_ip_threshold < 1 || f.failed_user_threshold < 1) return m.security_risk_policy_validation_attempt_min();
		if (f.temporary_block_minutes < 1) return m.security_risk_policy_validation_block_duration_min();
		return null;
	}

	async function save() {
		if (!form) return;
		const problem = validate(form);
		if (problem) {
			toast.error(problem);
			return;
		}
		if (!confirm(m.security_risk_policy_confirm_update())) {
			return;
		}
		saving = true;
		const [, err] = await updateRiskPolicy(tenantId, form);
		saving = false;
		if (err) {
			toast.error(m.security_risk_policy_save_error({ message: err.message }));
			return;
		}
		toast.success(m.security_risk_policy_save_success());
		queryClient.invalidateQueries({ queryKey: ['risk-policy', tenantId] });
	}
</script>

<svelte:head><title>{m.security_risk_policy_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` },
			{ name: m.nav_risk_policy(), path: `/tenants/${tenantId}/security/risk-policy` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_risk_policy()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.security_risk_policy_description()}
		</p>
	</div>

	<div class="mt-6 max-w-2xl">
		{#if policyQuery.isPending || !form}
			<Skeleton class="h-96 w-full rounded-xl" />
		{:else if policyQuery.isError}
			<ErrorAlert title={m.security_risk_policy_error_title()} message={policyQuery.error.message} />
		{:else}
			<div class="space-y-6">
				<label class="flex items-center gap-2 text-sm">
					<Checkbox bind:checked={form.enabled} />
					<span class="font-medium">{m.security_risk_policy_enabled_label()}</span>
				</label>

				<fieldset class="space-y-3 rounded-xl border p-4">
					<legend class="px-1 text-sm font-semibold">{m.security_risk_policy_section_thresholds()}</legend>
					{@render num(m.security_risk_policy_field_failed_ip(), 'failed_ip_threshold')}
					{@render num(m.security_risk_policy_field_failed_user(), 'failed_user_threshold')}
					{@render num(m.security_risk_policy_field_password_spraying(), 'password_spraying_user_threshold')}
					{@render num(m.security_risk_policy_field_tenant_attack(), 'tenant_attack_threshold')}
					{@render num(m.security_risk_policy_field_block_duration(), 'temporary_block_minutes')}
				</fieldset>

				<fieldset class="space-y-3 rounded-xl border p-4">
					<legend class="px-1 text-sm font-semibold">{m.security_risk_policy_section_score_bands()}</legend>
					{@render num(m.security_risk_policy_field_medium_threshold(), 'medium_risk_threshold')}
					{@render num(m.security_risk_policy_field_high_threshold(), 'high_risk_threshold')}
					{@render num(m.security_risk_policy_field_critical_threshold(), 'critical_risk_threshold')}
				</fieldset>

				<fieldset class="space-y-3 rounded-xl border p-4">
					<legend class="px-1 text-sm font-semibold">{m.security_risk_policy_section_enforcement()}</legend>
					<label class="flex items-center gap-2 text-sm">
						<Checkbox bind:checked={form.require_mfa_on_high_risk} />
						<span>{m.security_risk_policy_field_require_mfa()}</span>
					</label>
					<label class="flex items-center gap-2 text-sm">
						<Checkbox bind:checked={form.block_on_critical_risk} />
						<span>{m.security_risk_policy_field_block_critical()}</span>
					</label>
				</fieldset>

				<fieldset class="space-y-3 rounded-xl border p-4">
					<legend class="px-1 text-sm font-semibold">{m.security_risk_policy_section_scoring()}</legend>
					<label class="flex items-center justify-between gap-4 text-sm">
						<span class="text-muted-foreground">{m.security_risk_policy_scoring_mode_label()}</span>
						<select
							class="border-input bg-background rounded-md border px-3 py-2 text-sm"
							bind:value={form.scoring_mode}
						>
							<option value="rules">{m.security_risk_policy_scoring_mode_rules()}</option>
							<option value="shadow">{m.security_risk_policy_scoring_mode_shadow()}</option>
							<option value="hybrid">{m.security_risk_policy_scoring_mode_hybrid()}</option>
						</select>
					</label>
					<p class="text-muted-foreground text-xs">{m.security_risk_policy_scoring_help()}</p>
				</fieldset>

				<button
					class="bg-primary text-primary-foreground inline-flex items-center gap-1.5 rounded-md px-4 py-2 text-sm font-medium disabled:opacity-50"
					disabled={saving}
					onclick={save}
				>
					{#if saving}<LoadingSpinner class="size-4" />{:else}<SaveIcon class="size-4" />{/if}
					{saving ? m.security_risk_policy_saving() : m.security_risk_policy_save_button()}
				</button>
			</div>
		{/if}
	</div>
</MainContent>

{#snippet num(label: string, key: keyof Form)}
	<label class="flex items-center justify-between gap-4 text-sm">
		<span class="text-muted-foreground">{label}</span>
		<input
			type="number"
			min="1"
			class="border-input bg-background w-28 rounded-md border px-3 py-2 text-sm"
			value={form![key] as number}
			oninput={(e) => (form = { ...form!, [key]: Number(e.currentTarget.value) })}
		/>
	</label>
{/snippet}
