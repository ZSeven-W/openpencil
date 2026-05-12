// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

let isElectronValue = false;

vi.mock('@/utils/file-operations', () => ({
  isElectron: () => isElectronValue,
}));

vi.mock('@/components/cloud/auth-gate', () => ({
  AuthGate: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="auth-gate">{children}</div>
  ),
}));

vi.mock('@/components/cloud/cloud-file-library', () => ({
  CloudFileLibrary: () => <div>Cloud Library</div>,
}));

vi.mock('@/components/cloud/desktop-start-page', () => ({
  DesktopStartPage: () => <div>Desktop Start</div>,
}));

import { AppEntryPage } from './app-entry-page';

afterEach(() => {
  cleanup();
  isElectronValue = false;
});

describe('AppEntryPage', () => {
  it('renders the desktop start page in Electron', () => {
    isElectronValue = true;

    render(<AppEntryPage />);

    expect(screen.getByText('Desktop Start')).toBeTruthy();
    expect(screen.queryByTestId('auth-gate')).toBeNull();
  });

  it('renders the cloud file library directly on the web', () => {
    render(<AppEntryPage />);

    expect(screen.getByTestId('auth-gate')).toBeTruthy();
    expect(screen.getByText('Cloud Library')).toBeTruthy();
  });
});
