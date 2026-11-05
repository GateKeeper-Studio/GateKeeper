import type { MfaConfig, MfaMethod, NormalizedMfaConfig } from './types.js';

const MFA_METHODS: readonly MfaMethod[] = ['webauthn', 'totp', 'email'];

function normalizeMethod(method: string): MfaMethod {
  const normalized = method.trim().toLowerCase();
  if (!MFA_METHODS.includes(normalized as MfaMethod)) {
    throw new TypeError(`Unsupported MFA method: ${method}`);
  }

  return normalized as MfaMethod;
}

export function configureMfaMethods(config: MfaConfig): NormalizedMfaConfig {
  if (!config || typeof config !== 'object') {
    throw new TypeError('MFA config is required');
  }

  if (!Array.isArray(config.methods) || config.methods.length === 0) {
    throw new TypeError('MFA config requires at least one method');
  }

  const methods: MfaMethod[] = [];
  for (const method of config.methods) {
    if (typeof method !== 'string') {
      throw new TypeError('MFA methods must be strings');
    }

    const normalized = normalizeMethod(method);
    if (!methods.includes(normalized)) {
      methods.push(normalized);
    }
  }

  const defaultMethod = config.defaultMethod ? normalizeMethod(config.defaultMethod) : undefined;
  if (defaultMethod && !methods.includes(defaultMethod)) {
    throw new TypeError('MFA defaultMethod must be included in methods');
  }

  return defaultMethod ? { methods, defaultMethod } : { methods };
}

export const mfaMethods = Object.freeze({
  webauthn: 'webauthn',
  totp: 'totp',
  email: 'email'
} satisfies Record<MfaMethod, MfaMethod>);
