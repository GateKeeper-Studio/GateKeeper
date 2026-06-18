const MFA_METHODS = ['webauthn', 'totp', 'email'];

function normalizeMethod(method) {
  if (typeof method !== 'string') {
    throw new TypeError('MFA methods must be strings');
  }

  const normalized = method.trim().toLowerCase();
  if (!MFA_METHODS.includes(normalized)) {
    throw new TypeError(`Unsupported MFA method: ${method}`);
  }

  return normalized;
}

/**
 * Build server-side MFA preference hints for the hosted IdP login flow.
 *
 * These values are UX hints only. The IdP still enforces the OAuth client's
 * stored MFA policy and the user's enrolled factors before choosing a method.
 *
 * @param {{ methods: string[], defaultMethod?: string }} config
 * @returns {{ methods: string[], defaultMethod?: string }}
 */
export function configureMfaMethods(config) {
  if (!config || typeof config !== 'object') {
    throw new TypeError('MFA config is required');
  }

  if (!Array.isArray(config.methods) || config.methods.length === 0) {
    throw new TypeError('MFA config requires at least one method');
  }

  const methods = [];
  for (const method of config.methods) {
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
});
