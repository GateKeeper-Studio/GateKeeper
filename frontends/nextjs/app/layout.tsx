import type { ReactNode } from 'react';
import './globals.css';

export const metadata = {
  title: 'GateKeeper Next.js BFF',
  description: 'Confidential OAuth client (Backend-for-Frontend) for GateKeeper.'
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
