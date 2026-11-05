<script lang="ts">
  import {
    MfaConfiguration,
    defaultMfaMethodLabels,
    type MfaConfigurationClasses,
    type MfaConfigurationState,
    type MfaMethodEntry
  } from '@gatekeeper/sveltekit-bff/svelte';

  type Props = {
    initialMfa: MfaConfigurationState;
  };

  let { initialMfa }: Props = $props();

  const classes: MfaConfigurationClasses = {
    error: 'form-error',
    section: 'card',
    muted: 'muted',
    methodList: 'method-list',
    methodRow: 'method-row',
    methodInfo: 'method-info',
    methodName: 'method-name',
    methodMeta: 'method-meta',
    methodActions: 'method-actions',
    defaultLabel: 'check-row default-radio',
    actions: 'actions',
    primaryButton: 'primary',
    dangerButton: 'danger',
    totpSetup: 'totp-setup',
    qr: 'qr',
    totpSecret: 'totp-secret'
  };

  function methodLabel(entry: MfaMethodEntry): string {
    return entry.name || defaultMfaMethodLabels[entry.method] || entry.method;
  }

  function confirmRemove(entry: MfaMethodEntry): boolean {
    return confirm(`Remove "${methodLabel(entry)}"? You will no longer be able to sign in with it.`);
  }

  function promptRename(entry: MfaMethodEntry): string | null {
    return prompt('New name for this method', entry.name ?? '');
  }
</script>

<MfaConfiguration {initialMfa} {classes} {confirmRemove} {promptRename} />
