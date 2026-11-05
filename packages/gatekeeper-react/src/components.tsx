import type { ButtonHTMLAttributes, ReactNode } from 'react';
import type { SignInOptions } from '@gatekeeper/spa-oauth';
import { useGateKeeper } from './hooks.js';

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement>;

export interface SignInButtonProps extends ButtonProps {
  signInOptions?: SignInOptions;
  children?: ReactNode;
}

export function SignInButton({ signInOptions, children, onClick, ...rest }: SignInButtonProps): JSX.Element {
  const { signIn } = useGateKeeper();
  return (
    <button
      type="button"
      onClick={(event) => {
        onClick?.(event);
        if (!event.defaultPrevented) void signIn(signInOptions);
      }}
      {...rest}
    >
      {children ?? 'Sign in'}
    </button>
  );
}

export interface SignOutButtonProps extends ButtonProps {
  children?: ReactNode;
}

export function SignOutButton({ children, onClick, ...rest }: SignOutButtonProps): JSX.Element {
  const { signOut } = useGateKeeper();
  return (
    <button
      type="button"
      onClick={(event) => {
        onClick?.(event);
        if (!event.defaultPrevented) signOut();
      }}
      {...rest}
    >
      {children ?? 'Sign out'}
    </button>
  );
}

export interface AuthenticatedProps {
  children: ReactNode;
  /** Rendered while signed out. */
  fallback?: ReactNode;
}

/** Render children only when authenticated. */
export function Authenticated({ children, fallback = null }: AuthenticatedProps): JSX.Element {
  const { isAuthenticated } = useGateKeeper();
  return <>{isAuthenticated ? children : fallback}</>;
}
