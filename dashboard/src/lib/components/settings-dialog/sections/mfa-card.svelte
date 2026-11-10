<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import QRCode from 'qrcode';

	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import {
		confirmTotp,
		disableMfa,
		getMyMfa,
		regenerateRecoveryCodes,
		startTotp,
		type TotpEnrollment
	} from '$lib/services/admin/account';
	import type { ApiError } from '$lib/services/types';
	import RecoveryCodes from './recovery-codes.svelte';

	// Called once the dialog that turned two-step verification on has closed (the
	// admin has seen the recovery codes).
	let { onenabled }: { onenabled?: () => void } = $props();

	const queryClient = useQueryClient();
	const mfaQuery = createQuery(() => ({
		queryKey: ['account', 'mfa'],
		queryFn: async () => {
			const [data, err] = await getMyMfa();
			if (err) throw err;
			return data;
		}
	}));
	const status = $derived(mfaQuery.data);

	// One dialog, three flows:
	//   setup:      password -> scan + code -> recovery codes
	//   regenerate: password -> recovery codes
	//   disable:    password + code
	type Flow = 'setup' | 'regenerate' | 'disable';
	type Step = 'password' | 'scan' | 'codes';

	let open = $state(false);
	let flow = $state<Flow>('setup');
	let step = $state<Step>('password');
	let password = $state('');
	let code = $state('');
	let busy = $state(false);
	let error = $state('');
	let enrollment = $state<TotpEnrollment | null>(null);
	let qrDataUrl = $state('');
	let codes = $state<string[]>([]);
	let enabledNow = $state(false);

	function begin(next: Flow) {
		flow = next;
		step = 'password';
		password = '';
		code = '';
		error = '';
		enrollment = null;
		qrDataUrl = '';
		codes = [];
		open = true;
	}

	// Closing: pick up whatever changed. The secrets this dialog held (seed, codes,
	// typed password) are dropped once the close animation has finished, so the
	// content does not empty out while it is still on screen.
	function onOpenChange(isOpen: boolean) {
		if (!isOpen) queryClient.invalidateQueries({ queryKey: ['account', 'mfa'] });
	}

	function onOpenChangeComplete(isOpen: boolean) {
		if (!isOpen && enabledNow) {
			enabledNow = false;
			onenabled?.();
		}
		if (!isOpen) {
			password = '';
			code = '';
			enrollment = null;
			qrDataUrl = '';
			codes = [];
		}
	}

	function close() {
		open = false;
		onOpenChange(false);
	}

	function messageOf(err: ApiError) {
		return err.data?.message || err.message || m.settings_mfa_error();
	}

	async function submitPassword(event: SubmitEvent) {
		event.preventDefault();
		if (password.length === 0) {
			error = m.settings_mfa_password_required();
			return;
		}
		error = '';
		busy = true;

		if (flow === 'setup') {
			const [data, err] = await startTotp(password);
			busy = false;
			if (err || !data) {
				error = err ? messageOf(err) : m.settings_mfa_error();
				return;
			}
			password = '';
			enrollment = data;
			qrDataUrl = await QRCode.toDataURL(data.otpauthUrl, { margin: 1, width: 192 });
			step = 'scan';
			return;
		}

		if (flow === 'regenerate') {
			const [data, err] = await regenerateRecoveryCodes(password);
			busy = false;
			if (err || !data) {
				error = err ? messageOf(err) : m.settings_mfa_error();
				return;
			}
			password = '';
			codes = data.recoveryCodes;
			step = 'codes';
			return;
		}

		// disable
		if (code.trim().length === 0) {
			busy = false;
			error = m.settings_mfa_code_required();
			return;
		}
		const [err] = await disableMfa(password, code.trim());
		busy = false;
		if (err) {
			error = messageOf(err);
			code = '';
			return;
		}
		toast.success(m.settings_mfa_disabled_toast());
		close();
	}

	async function submitCode(event: SubmitEvent) {
		event.preventDefault();
		if (code.trim().length === 0) {
			error = m.settings_mfa_code_required();
			return;
		}
		error = '';
		busy = true;
		const [data, err] = await confirmTotp(code.trim());
		busy = false;
		if (err || !data) {
			error = err ? messageOf(err) : m.settings_mfa_error();
			code = '';
			return;
		}
		enrollment = null;
		qrDataUrl = '';
		codes = data.recoveryCodes;
		step = 'codes';
		enabledNow = true;
		toast.success(m.settings_mfa_enabled_toast());
	}
</script>

<Card.Root class="mt-6 max-w-[640px]">
	<Card.Header>
		<Card.Title class="flex items-center gap-2">
			<ShieldCheckIcon class="size-4" />
			{m.settings_mfa_title()}
			{#if status}
				{#if status.totpEnabled}
					<Badge variant="default">{m.settings_mfa_status_on()}</Badge>
				{:else}
					<Badge variant="secondary">{m.settings_mfa_status_off()}</Badge>
				{/if}
			{/if}
		</Card.Title>
		<Card.Description>{m.settings_mfa_description()}</Card.Description>
	</Card.Header>
	<Card.Content class="space-y-4">
		{#if mfaQuery.isPending}
			<div class="flex items-center gap-2 text-sm text-muted-foreground">
				<LoadingSpinner />
				{m.settings_mfa_loading()}
			</div>
		{:else if mfaQuery.isError}
			<ErrorAlert title={m.settings_mfa_load_error()} message={mfaQuery.error.message} />
		{:else if status?.totpEnabled}
			<div class="grid gap-1 text-sm">
				{#if status.enabledAt}
					<span class="text-muted-foreground">
						{m.settings_mfa_enabled_since({
							date: new Date(status.enabledAt).toLocaleDateString()
						})}
					</span>
				{/if}
				<span>{m.settings_mfa_codes_left({ count: status.recoveryCodesRemaining })}</span>
				{#if status.recoveryCodesRemaining <= 3}
					<span class="font-medium text-destructive">{m.settings_mfa_codes_low()}</span>
				{/if}
			</div>
			<div class="flex flex-wrap gap-2">
				<Button variant="outline" onclick={() => begin('regenerate')}>
					{m.settings_mfa_regenerate_button()}
				</Button>
				<Button variant="destructive" onclick={() => begin('disable')}>
					{m.settings_mfa_disable_button()}
				</Button>
			</div>
		{:else}
			<p class="text-sm text-muted-foreground">{m.settings_mfa_off_hint()}</p>
			<Button onclick={() => begin('setup')}>{m.settings_mfa_setup_button()}</Button>
		{/if}
	</Card.Content>
</Card.Root>

<Dialog.Root bind:open {onOpenChange} {onOpenChangeComplete}>
	<Dialog.Content class="sm:max-w-md">
		{#if step === 'password'}
			<Dialog.Header>
				<Dialog.Title>
					{#if flow === 'setup'}
						{m.settings_mfa_setup_button()}
					{:else if flow === 'regenerate'}
						{m.settings_mfa_regenerate_title()}
					{:else}
						{m.settings_mfa_disable_title()}
					{/if}
				</Dialog.Title>
				<Dialog.Description>
					{#if flow === 'regenerate'}
						{m.settings_mfa_regenerate_description()}
					{:else if flow === 'disable'}
						{m.settings_mfa_disable_description()}
					{:else}
						{m.settings_mfa_password_step()}
					{/if}
				</Dialog.Description>
			</Dialog.Header>
			<form onsubmit={submitPassword} class="grid gap-4">
				<div class="space-y-2">
					<Label for="mfa-password">{m.settings_mfa_password_label()}</Label>
					<Input
						id="mfa-password"
						type="password"
						autocomplete="current-password"
						bind:value={password}
					/>
				</div>
				{#if flow === 'disable'}
					<div class="space-y-2">
						<Label for="mfa-disable-code">{m.settings_mfa_disable_code_label()}</Label>
						<Input
							id="mfa-disable-code"
							type="text"
							autocomplete="one-time-code"
							spellcheck="false"
							maxlength={32}
							class="font-mono tracking-widest"
							bind:value={code}
						/>
					</div>
				{/if}
				{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
				<Dialog.Footer>
					<Button type="button" variant="outline" disabled={busy} onclick={close}>
						{m.settings_mfa_cancel()}
					</Button>
					<Button
						type="submit"
						variant={flow === 'disable' ? 'destructive' : 'default'}
						disabled={busy}
						class="relative"
					>
						{#if busy}<LoadingSpinner class="absolute left-3" />{/if}
						{flow === 'disable' ? m.settings_mfa_disable_button() : m.settings_mfa_continue()}
					</Button>
				</Dialog.Footer>
			</form>
		{:else if step === 'scan' && enrollment}
			<Dialog.Header>
				<Dialog.Title>{m.settings_mfa_scan_title()}</Dialog.Title>
				<Dialog.Description>{m.settings_mfa_scan_description()}</Dialog.Description>
			</Dialog.Header>
			<form onsubmit={submitCode} class="grid gap-4">
				{#if qrDataUrl}
					<img
						src={qrDataUrl}
						alt={m.settings_mfa_qr_alt()}
						width="192"
						height="192"
						class="mx-auto rounded-md border bg-white p-2"
					/>
				{/if}
				<div class="space-y-1 text-center">
					<p class="text-xs text-muted-foreground">{m.settings_mfa_secret_label()}</p>
					<code class="font-mono text-sm tracking-wider break-all select-all">
						{enrollment.secretBase32}
					</code>
				</div>
				<div class="space-y-2">
					<Label for="mfa-confirm-code">{m.settings_mfa_code_label()}</Label>
					<Input
						id="mfa-confirm-code"
						type="text"
						inputmode="numeric"
						autocomplete="one-time-code"
						maxlength={6}
						placeholder="123456"
						class="font-mono tracking-widest"
						bind:value={code}
					/>
				</div>
				{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
				<Dialog.Footer>
					<Button type="button" variant="outline" disabled={busy} onclick={close}>
						{m.settings_mfa_cancel()}
					</Button>
					<Button type="submit" disabled={busy} class="relative">
						{#if busy}<LoadingSpinner class="absolute left-3" />{/if}
						{m.settings_mfa_confirm_button()}
					</Button>
				</Dialog.Footer>
			</form>
		{:else if step === 'codes'}
			<Dialog.Header>
				<Dialog.Title>{m.settings_mfa_codes_title()}</Dialog.Title>
				<Dialog.Description>{m.settings_mfa_codes_description()}</Dialog.Description>
			</Dialog.Header>
			<RecoveryCodes {codes} />
			<Dialog.Footer>
				<Button onclick={close}>{m.settings_mfa_codes_done()}</Button>
			</Dialog.Footer>
		{/if}
	</Dialog.Content>
</Dialog.Root>
