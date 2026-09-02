import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke } from '@tauri-apps/api/core';

export type { Update };

export type UpdateState =
	| { status: 'idle' }
	| { status: 'checking' }
	| { status: 'available'; update: Update }
	| { status: 'downloading'; progress: number; contentLength: number | null }
	| { status: 'downloaded' }
	| { status: 'installing' }
	| { status: 'installed'; version: string }
	| { status: 'error'; message: string };

export async function getAppVersion(): Promise<string> {
	try {
		return await invoke<string>('get_app_version');
	} catch {
		return 'web';
	}
}

// check for updates without auto-installing; returns null when up to date
export async function checkForUpdate(): Promise<Update | null> {
	return check();
}

// download and install the given update, reporting progress through onProgress (0..1)
export async function downloadAndInstall(
	update: Update,
	onProgress: (fraction: number) => void
): Promise<void> {
	let downloaded = 0;
	let contentLength = 0;
	await update.downloadAndInstall((event) => {
		if (event.event === 'Started') {
			contentLength = event.data.contentLength ?? 0;
		} else if (event.event === 'Progress') {
			downloaded += event.data.chunkLength;
			if (contentLength > 0) {
				onProgress(Math.min(1, downloaded / contentLength));
			}
		}
	});
}

// relaunch the app so the freshly installed version takes effect
export async function relaunchApp(): Promise<void> {
	await relaunch();
}
