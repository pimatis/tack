import { getDb, isTauri } from '$lib/db/client';
import { invoke } from '@tauri-apps/api/core';
import type { TaskAttachment } from '$lib/types/attachment';

type CreateAttachmentInput = {
	taskId: string;
	fileName: string;
	fileData: string;
	mimeType: string;
	fileSize: number;
};

const METADATA_COLUMNS = `
	id,
	task_id AS taskId,
	file_name AS fileName,
	file_path AS filePath,
	mime_type AS mimeType,
	file_size AS fileSize,
	created_at AS createdAt
`;

export async function create(input: CreateAttachmentInput): Promise<TaskAttachment> {
	try {
		const db = await getDb();
		const id = crypto.randomUUID();
		const now = new Date().toISOString();

		// save file to disk (rust command in the app, http upload on the live site)
		const filePath = await saveFileData(id, input.fileData);

		await db.execute(
			`INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, file_size, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
			[id, input.taskId, input.fileName, filePath, input.mimeType, input.fileSize, now]
		);

		return {
			id,
			taskId: input.taskId,
			fileName: input.fileName,
			filePath,
			mimeType: input.mimeType,
			fileSize: input.fileSize,
			createdAt: now
		};
	} catch (error) {
		throw new Error('Failed to create attachment', { cause: error });
	}
}

async function saveFileData(id: string, fileData: string): Promise<string> {
	if (isTauri()) {
		return await invoke<string>('save_attachment', { id, fileData });
	}
	const res = await fetch(`/api/attachment/${id}`, {
		method: 'PUT',
		body: dataUrlToBytes(fileData)
	});
	if (!res.ok) throw new Error(await res.text());
	return '';
}

function dataUrlToBytes(dataUrl: string): Uint8Array {
	const base64 = dataUrl.split(',')[1] ?? dataUrl;
	const bin = atob(base64);
	const bytes = new Uint8Array(bin.length);
	for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
	return bytes;
}

export async function findByTaskId(taskId: string): Promise<TaskAttachment[]> {
	try {
		const db = await getDb();
		return await db.select<TaskAttachment[]>(
			`SELECT ${METADATA_COLUMNS} FROM task_attachments WHERE task_id = $1 ORDER BY created_at ASC`,
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load attachments', { cause: error });
	}
}

// load file data on demand for display
export async function getAttachmentData(id: string, mimeType: string): Promise<string> {
	if (isTauri()) {
		return await invoke<string>('read_attachment', { id, mimeType });
	}
	const res = await fetch(`/api/attachment/${id}?mime=${encodeURIComponent(mimeType)}`);
	if (!res.ok) throw new Error(await res.text());
	const blob = await res.blob();
	return await new Promise((resolve) => {
		const reader = new FileReader();
		reader.onload = () => resolve(reader.result as string);
		reader.readAsDataURL(blob);
	});
}

// copy attachment file to a user-selected path, or start a plain browser download
export async function downloadAttachment(
	id: string,
	fileName: string,
	destPath?: string
): Promise<void> {
	if (isTauri() && destPath) {
		await invoke('download_attachment', { id, destPath });
		return;
	}
	const res = await fetch(`/api/attachment/${id}`);
	if (!res.ok) throw new Error(await res.text());
	const blob = await res.blob();
	const url = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = url;
	link.download = fileName;
	document.body.appendChild(link);
	link.click();
	link.remove();
	URL.revokeObjectURL(url);
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const result = await db.execute('DELETE FROM task_attachments WHERE id = $1', [id]);
		if (result.rowsAffected > 0) {
			if (isTauri()) {
				void invoke('delete_attachment', { id });
			} else {
				void fetch(`/api/attachment/${id}`, { method: 'DELETE' });
			}
		}
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete attachment', { cause: error });
	}
}
