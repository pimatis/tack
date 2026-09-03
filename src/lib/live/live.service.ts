import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '$lib/db/client';
import { getSettings, setSettings } from '$lib/stores/settings';

export type LiveStatus = { port: number; url: string };

// marks the one-time session reset; in sessionStorage so it survives webview
// reloads (dev hot-reload must not stop a running live server)
const LIVE_SESSION_KEY = 'tack-live-session-reset';

// keep the embedded server in sync with the liveEnabled/livePort settings,
// including changes made from the cli (tack settings set liveEnabled true)
export function startLiveManager(): () => void {
	if (!isTauri()) return () => {};
	// live is session-scoped: the persisted flag must never auto-start the
	// server on launch, so reset it once per webview session
	if (!sessionStorage.getItem(LIVE_SESSION_KEY)) {
		sessionStorage.setItem(LIVE_SESSION_KEY, '1');
		if (getSettings().liveEnabled) setSettings({ liveEnabled: false });
	}
	const reconcile = () => void reconcileLive();
	window.addEventListener('settings-changed', reconcile);
	void reconcileLive();
	return () => window.removeEventListener('settings-changed', reconcile);
}

let reconciling = false;

async function reconcileLive(): Promise<void> {
	if (reconciling) return;
	reconciling = true;
	try {
		const { liveEnabled, livePort } = getSettings();
		const status = await getLiveStatus();
		if (liveEnabled && status && status.port === livePort) return;
		if (!liveEnabled && !status) return;
		if (liveEnabled) {
			if (status) await invoke('live_stop');
			const next = await invoke<LiveStatus>('live_start', { port: livePort });
			dispatchLiveStatus(next);
		} else {
			await invoke('live_stop');
			dispatchLiveStatus(null);
		}
	} catch (error) {
		// start failed (e.g. port busy): flip the toggle off so the ui matches
		// the real server state instead of showing "on" while nothing listens
		setSettings({ liveEnabled: false });
		window.dispatchEvent(new CustomEvent('live-error-changed', { detail: String(error) }));
	} finally {
		reconciling = false;
	}
}

export async function getLiveStatus(): Promise<LiveStatus | null> {
	try {
		return await invoke<LiveStatus | null>('live_status');
	} catch {
		return null;
	}
}

function dispatchLiveStatus(status: LiveStatus | null): void {
	window.dispatchEvent(new CustomEvent('live-status-changed', { detail: status }));
}
