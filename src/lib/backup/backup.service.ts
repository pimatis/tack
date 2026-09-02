import { invoke } from '@tauri-apps/api/core';
import { getSettings } from '$lib/stores/settings';
import { getDb, isTauri, resetDb } from '$lib/db/client';

export type BackupInfo = {
	name: string;
	createdAt: string;
	sizeBytes: number;
};

// browser mode: talk to the backup endpoints of the embedded live server
async function http<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(path, init);
	if (!res.ok) throw new Error(await res.text());
	return (await res.json()) as T;
}

export function listBackups(): Promise<BackupInfo[]> {
	if (isTauri()) return invoke<BackupInfo[]>('list_backups');
	return http<{ backups: BackupInfo[] }>('/api/backups').then((d) => d.backups);
}

export function createBackup(keep?: number): Promise<string> {
	if (isTauri()) return invoke<string>('create_backup', { keep: keep ?? 7 });
	return http<{ name: string }>('/api/backups', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ keep: keep ?? 7 })
	}).then((d) => d.name);
}

export async function restoreBackup(name: string): Promise<void> {
	if (isTauri()) {
		await invoke('restore_backup', { name });
	} else {
		await http(`/api/backups/${encodeURIComponent(name)}/restore`, { method: 'POST' });
	}
	// drop cached pool so the reload reopens the restored files
	try {
		await (await getDb()).close();
	} catch {
		// already closed
	}
	resetDb();
}

export function deleteBackup(name: string): Promise<void> {
	if (isTauri()) return invoke<void>('delete_backup', { name });
	return http(`/api/backups/${encodeURIComponent(name)}`, { method: 'DELETE' });
}

async function runScheduledBackup(): Promise<string | null> {
	const { backupEnabled, backupIntervalHours, backupKeepCount } = getSettings();
	if (!backupEnabled) return null;
	const backups = await listBackups().catch(() => [] as BackupInfo[]);
	const latest = backups[0];
	if (
		latest &&
		Date.now() - new Date(latest.createdAt).getTime() < backupIntervalHours * 3_600_000
	) {
		return null;
	}
	return createBackup(backupKeepCount);
}

export function startBackupScheduler(): () => void {
	// scheduled backups run only in the desktop app; every open live tab would
	// otherwise run its own duplicate scheduler
	if (!isTauri()) return () => {};
	void runScheduledBackup().catch(() => {});
	const interval = setInterval(
		() => {
			void runScheduledBackup().catch(() => {});
		},
		15 * 60 * 1000
	);
	return () => clearInterval(interval);
}
