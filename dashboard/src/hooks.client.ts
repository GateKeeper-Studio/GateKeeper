import { z } from 'zod';

// The identity provider serves this app with a CSP that has no 'unsafe-eval'.
// zod's compiled ("JIT") validators need `new Function`, and its feature probe
// alone makes the browser report a CSP violation. Interpreted validation is plenty
// for form-sized inputs.
z.config({ jitless: true });
