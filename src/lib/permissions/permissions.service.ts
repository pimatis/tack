import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '$lib/db/client';

// 'unsupported' means macOS outside a bundled app (a `tauri dev` binary has no
// bundle id, so the system notification center cannot be used)
export type PermissionState = 'granted' | 'prompt' | 'denied' | 'unsupported';

export async function notificationPermission(): Promise<PermissionState> {
	if (!isTauri()) return 'unsupported';
	try {
		const state = await invoke<string>('notification_permission');
		if (state === 'granted' || state === 'denied' || state === 'prompt') return state;
		return 'unsupported';
	} catch {
		return 'unsupported';
	}
}

// triggers the os prompt, then re-reads the status: the prompt resolves
// asynchronously, so a short wait covers the common case
export async function requestNotificationPermission(): Promise<boolean> {
	if (!isTauri()) return false;
	try {
		await invoke('request_notification_permission');
		await new Promise((resolve) => setTimeout(resolve, 1200));
		return (await notificationPermission()) === 'granted';
	} catch {
		return false;
	}
}

// deep link straight to the os notification pane (macOS)
export async function openNotificationSettings(): Promise<void> {
	if (!isTauri()) return;
	await invoke('open_notification_settings');
}
