<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import KeyIcon from '@lucide/svelte/icons/key';
	import RefreshCcwIcon from '@lucide/svelte/icons/refresh-ccw';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { m } from '$lib/paraglide/messages.js';

	let { setPassword }: { setPassword: (password: string) => void } = $props();

	let open = $state(false);
	let isCopied = $state(false);
	let isPasswordVisible = $state(false);
	let strongPassword = $state('');

	const PASSWORD_LENGTH = 32;
	const CHAR_SET =
		'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?';

	function generatePassword(): string {
		// crypto.getRandomValues gives cryptographically strong randomness,
		// unlike Math.random which is fine for UI but not for secrets.
		const randomValues = new Uint32Array(PASSWORD_LENGTH);
		crypto.getRandomValues(randomValues);

		let password = '';
		for (let i = 0; i < PASSWORD_LENGTH; i++) {
			password += CHAR_SET[randomValues[i] % CHAR_SET.length];
		}
		return password;
	}

	function copyToClipboard() {
		navigator.clipboard.writeText(strongPassword);
		isCopied = true;
		setTimeout(() => (isCopied = false), 1000);
	}

	function usePassword() {
		setPassword(strongPassword);
		open = false;
	}

	// Generate a fresh password each time the dialog opens.
	$effect(() => {
		if (open && strongPassword === '') {
			strongPassword = generatePassword();
		}
	});
</script>

<Dialog.Root bind:open>
	<Dialog.Trigger class="w-fit text-sm text-blue-500 underline-offset-2 hover:underline">
		{m.create_tenant_generate_password()}
	</Dialog.Trigger>

	<Dialog.Content class="sm:max-w-[38rem]">
		<Dialog.Header>
			<Dialog.Title>{m.create_tenant_generate_password()}</Dialog.Title>

			<Dialog.Description>
				{m.create_tenant_generate_password_description()}
			</Dialog.Description>
		</Dialog.Header>

		<div
			class="mt-2 flex items-center justify-between rounded-[10px] border border-gray-200 bg-background p-2 px-4 shadow-inner brightness-90 dark:border-gray-700"
		>
			<strong class="text-[1.15rem] font-bold tracking-widest break-all text-primary">
				{isPasswordVisible ? strongPassword : strongPassword.replace(/./g, '*')}
			</strong>

			<div class="ml-3 flex shrink-0 items-center gap-3">
				<button
					type="button"
					onclick={() => (isPasswordVisible = !isPasswordVisible)}
					title={m.create_tenant_show_password_tooltip()}
					class="flex w-fit items-center justify-center"
				>
					{#if isPasswordVisible}
						<EyeOffIcon size={24} />
					{:else}
						<EyeIcon size={24} />
					{/if}
				</button>

				<button
					type="button"
					onclick={() => (strongPassword = generatePassword())}
					title={m.create_tenant_regenerate_password_tooltip()}
					class="flex w-fit items-center justify-center transition-all active:scale-90"
				>
					<RefreshCcwIcon size={24} />
				</button>
			</div>
		</div>

		<div class="ml-auto flex gap-2">
			<Button variant="outline" type="button" onclick={copyToClipboard}>
				{#if isCopied}
					<CheckIcon size={20} />
					{m.create_tenant_copied()}
				{:else}
					<CopyIcon size={20} />
					{m.create_tenant_copy()}
				{/if}
			</Button>

			<Button type="button" onclick={usePassword}>
				<KeyIcon size={20} />
				{m.create_tenant_use_password()}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
