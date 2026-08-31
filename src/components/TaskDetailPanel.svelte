<script lang="ts">
	import { onMount } from 'svelte';
	import { update, togglePin } from '$lib/repositories/task.repository';
	import {
		create as createAttachment,
		findByTaskId,
		remove as removeAttachment,
		getAttachmentData,
		downloadAttachment
	} from '$lib/repositories/attachment.repository';
	import { setTaskLabels, findLabelIdsByTaskId } from '$lib/repositories/label.repository';
	import {
		findByTaskId as findSubtasks,
		create as createSubtask,
		toggle as toggleSubtask,
		remove as removeSubtask,
		rename as renameSubtask,
		reorder as reorderSubtasks
	} from '$lib/repositories/subtask.repository';
	import { findByTaskId as findActivity } from '$lib/repositories/activity.repository';
	import { log as logActivity } from '$lib/repositories/activity.repository';
	import type { Label } from '$lib/types/label';
	import type { TaskAttachment } from '$lib/types/attachment';
	import type { Subtask } from '$lib/types/subtask';
	import type { ActivityLog } from '$lib/types/activity';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import { fade, fly, scale } from 'svelte/transition';
	import { sortableItem, reorderArray, type DragDropState } from '$lib/dnd';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import { Progress } from '$lib/components/ui/progress/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import LabelSelector from './LabelSelector.svelte';
	import StatusIcon from './StatusIcon.svelte';
	import MarkdownRenderer from './MarkdownRenderer.svelte';
	import DueDatePicker from './DueDatePicker.svelte';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';

	type Props = {
		open?: boolean;
		task: Task | null;
		prefix: string;
		labels: Label[];
		onUpdated?: (task: Task) => void;
		onLabelCreated?: (label: Label) => void;
		onLabelUpdated?: (label: Label) => void;
		onLabelRemoved?: (id: string) => void;
	};

	let {
		open = $bindable(false),
		task,
		prefix,
		labels,
		onUpdated,
		onLabelCreated,
		onLabelUpdated,
		onLabelRemoved
	}: Props = $props();

	let title = $state('');
	let description = $state('');
	let status = $state<TaskStatus>('todo');
	let priority = $state('0');
	let dueDate = $state<string>('');
	let endDate = $state<string>('');
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let previewMode = $state(false);
	let titleRef = $state<HTMLInputElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let attachments = $state<TaskAttachment[]>([]);
	let attachmentUrls = $state<Record<string, string>>({});
	let lightboxUrl = $state<string | null>(null);
	let lightboxRef = $state<HTMLDivElement | null>(null);
	let pendingAttachments = $state<
		{ fileName: string; fileData: string; mimeType: string; fileSize: number }[]
	>([]);
	let uploading = $state(false);
	let loadingAttachments = $state(false);
	let selectedLabelIds = $state<string[]>([]);
	let panelRef = $state<HTMLElement | null>(null);

	// subtasks
	let subtasks = $state<Subtask[]>([]);
	let newSubtaskTitle = $state('');
	let loadingSubtasks = $state(false);
	let editingSubtaskId = $state<string | null>(null);
	let editingSubtaskTitle = $state('');

	// activity
	let activities = $state<ActivityLog[]>([]);

	const statusLabels: Record<TaskStatus, string> = {
		todo: 'Todo',
		in_progress: 'In progress',
		done: 'Done',
		canceled: 'Canceled'
	};

	const priorityLabels: Record<string, string> = {
		'0': 'No priority',
		'1': 'Urgent',
		'2': 'High',
		'3': 'Medium',
		'4': 'Low'
	};

	const priorityConfig: Record<number, { label: string }> = {
		0: { label: 'No priority' },
		1: { label: 'Urgent' },
		2: { label: 'High' },
		3: { label: 'Medium' },
		4: { label: 'Low' }
	};

	const statusOrder: TaskStatus[] = ['todo', 'in_progress', 'done', 'canceled'];

	let subtaskProgress = $derived(
		subtasks.length > 0
			? Math.round((subtasks.filter((s) => s.completed).length / subtasks.length) * 100)
			: 0
	);

	$effect(() => {
		if (!open || !task) return;
		title = task.title;
		description = task.description ?? '';
		status = task.status;
		priority = String(task.priority);
		dueDate = task.dueDate ?? '';
		endDate = task.endDate ?? '';
		error = null;
		previewMode = false;
		pendingAttachments = [];
		attachments = [];
		attachmentUrls = {};
		selectedLabelIds = [];
		newSubtaskTitle = '';
		void loadAttachments(task.id);
		void loadTaskLabels(task.id);
		void loadSubtasks(task.id);
		void loadActivity(task.id);
	});

	$effect(() => {
		// keep focus inside the panel, also return it after the lightbox closes
		if (open && !lightboxUrl) requestAnimationFrame(() => titleRef?.focus());
	});

	// escape closes the lightbox first, then the panel (independent of focus)
	$effect(() => {
		if (!open) return;
		const onKeydown = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && lightboxUrl) {
				lightboxUrl = null;
				e.stopPropagation();
			}
		};
		document.addEventListener('keydown', onKeydown, { capture: true });
		return () => document.removeEventListener('keydown', onKeydown, { capture: true });
	});

	// clicks on the lightbox close it first, never the panel (independent of focus)
	$effect(() => {
		if (!open) return;
		const onPointerDown = (e: PointerEvent) => {
			if (!lightboxUrl || !lightboxRef) return;
			if (lightboxRef.contains(e.target as Node)) {
				e.stopPropagation();
				lightboxUrl = null;
			}
		};
		document.addEventListener('pointerdown', onPointerDown, { capture: true });
		return () => document.removeEventListener('pointerdown', onPointerDown, { capture: true });
	});

	async function loadTaskLabels(taskId: string) {
		try {
			selectedLabelIds = await findLabelIdsByTaskId(taskId);
		} catch {
			selectedLabelIds = [];
		}
	}

	async function loadAttachments(taskId: string) {
		loadingAttachments = true;
		try {
			attachments = await findByTaskId(taskId);
			// lazy load image data for display
			for (const att of attachments) {
				if (isImage(att.mimeType) && !attachmentUrls[att.id]) {
					try {
						const data = await getAttachmentData(att.id, att.mimeType);
						attachmentUrls = { ...attachmentUrls, [att.id]: data };
					} catch {
						// skip failed loads
					}
				}
			}
		} catch {
			// attachments are optional
		} finally {
			loadingAttachments = false;
		}
	}

	async function loadSubtasks(taskId: string) {
		loadingSubtasks = true;
		try {
			subtasks = await findSubtasks(taskId);
		} catch {
			subtasks = [];
		} finally {
			loadingSubtasks = false;
		}
	}

	async function loadActivity(taskId: string) {
		try {
			activities = await findActivity(taskId);
		} catch {
			activities = [];
		}
	}

	function isImage(mimeType: string) {
		return mimeType.startsWith('image/');
	}

	function fileToBase64(file: File): Promise<string> {
		return new Promise((resolve) => {
			const reader = new FileReader();
			reader.onload = () => resolve(reader.result as string);
			reader.readAsDataURL(file);
		});
	}

	async function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		if (!input.files?.length) return;

		uploading = true;
		try {
			for (const file of Array.from(input.files)) {
				if (!isImage(file.type)) {
					error = `${file.name} is not an image`;
					continue;
				}
				if (file.size > 10 * 1024 * 1024) {
					error = `${file.name} exceeds 10MB limit`;
					continue;
				}
				const fileData = await fileToBase64(file);
				pendingAttachments = [
					...pendingAttachments,
					{
						fileName: file.name,
						fileData,
						mimeType: file.type || 'image/png',
						fileSize: file.size
					}
				];
			}
		} catch {
			error = 'Failed to read file';
		} finally {
			uploading = false;
			input.value = '';
		}
	}

	function removePending(index: number) {
		pendingAttachments = pendingAttachments.filter((_, i) => i !== index);
	}

	async function deleteExisting(id: string) {
		try {
			await removeAttachment(id);
			attachments = attachments.filter((a) => a.id !== id);
			const rest = { ...attachmentUrls };
			delete rest[id];
			attachmentUrls = rest;
			if (task) void logActivity(task.id, 'attachment_removed', undefined, undefined, undefined);
			if (task) void loadActivity(task.id);
		} catch {
			error = 'Failed to delete attachment';
		}
	}

	async function handleDownloadAttachment(att: TaskAttachment) {
		try {
			const ext = att.fileName.split('.').pop() ?? '';
			const filters: { name: string; extensions: string[] }[] = [];
			if (ext) {
				filters.push({ name: 'Image', extensions: [ext] });
			}
			filters.push({ name: 'All files', extensions: ['*'] });
			const destPath = await saveDialog({
				defaultPath: att.fileName,
				filters
			});
			if (!destPath) return;
			await downloadAttachment(att.id, destPath);
		} catch {
			error = 'Failed to download attachment';
		}
	}

	async function handleAddSubtask() {
		if (!task || !newSubtaskTitle.trim()) return;
		try {
			const subtask = await createSubtask(task.id, newSubtaskTitle.trim());
			subtasks = [...subtasks, subtask];
			newSubtaskTitle = '';
			void logActivity(task.id, 'subtask_added', undefined, undefined, subtask.title);
			void loadActivity(task.id);
		} catch {
			error = 'Failed to add subtask';
		}
	}

	async function handleToggleSubtask(subtask: Subtask) {
		try {
			await toggleSubtask(subtask.id, !subtask.completed);
			subtasks = subtasks.map((s) => (s.id === subtask.id ? { ...s, completed: !s.completed } : s));
			if (task) {
				void logActivity(
					task.id,
					subtask.completed ? 'subtask_uncompleted' : 'subtask_completed',
					undefined,
					undefined,
					subtask.title
				);
				void loadActivity(task.id);
			}
		} catch {
			error = 'Failed to update subtask';
		}
	}

	async function handleRemoveSubtask(id: string) {
		try {
			await removeSubtask(id);
			subtasks = subtasks.filter((s) => s.id !== id);
			if (task) {
				void logActivity(task.id, 'subtask_removed');
				void loadActivity(task.id);
			}
		} catch {
			error = 'Failed to delete subtask';
		}
	}

	function startEditSubtask(subtask: Subtask) {
		editingSubtaskId = subtask.id;
		editingSubtaskTitle = subtask.title;
	}

	function cancelEditSubtask() {
		editingSubtaskId = null;
		editingSubtaskTitle = '';
	}

	async function handleRenameSubtask(id: string) {
		const title = editingSubtaskTitle.trim();
		if (!title) {
			cancelEditSubtask();
			return;
		}
		try {
			await renameSubtask(id, title);
			subtasks = subtasks.map((s) => (s.id === id ? { ...s, title } : s));
			editingSubtaskId = null;
			editingSubtaskTitle = '';
		} catch {
			error = 'Failed to rename subtask';
		}
	}

	async function handleSubtaskDrop(state: DragDropState<Subtask>, targetSubtask: Subtask) {
		const dragged = state.draggedItem;
		if (!dragged || !task || !state.dropPosition) return;

		subtasks = reorderArray(subtasks, dragged, targetSubtask, state.dropPosition);
		try {
			await reorderSubtasks(
				task.id,
				subtasks.map((s) => s.id)
			);
		} catch {
			error = 'Failed to reorder subtasks';
		}
	}

	function close() {
		open = false;
	}

	onMount(() => {
		const registry = getShortcutRegistry();

		const unregisterClose = registry.register({
			id: 'close',
			enabled: () => open,
			run: () => close()
		});

		const unregisterSave = registry.register({
			id: 'save-task',
			enabled: () => open,
			run: () => void handleSubmit()
		});

		return () => {
			unregisterClose();
			unregisterSave();
		};
	});

	async function handleTogglePin() {
		if (!task) return;
		try {
			const updated = await togglePin(task.id);
			if (updated && onUpdated) onUpdated(updated);
		} catch {
			error = 'Failed to toggle pin';
		}
	}

	async function handleSubmit() {
		if (!task || !title.trim()) {
			error = 'Title is required';
			return;
		}
		submitting = true;
		try {
			const updated = await update(task.id, {
				title: title.trim(),
				description: description.trim() || null,
				status,
				priority: Number(priority) as TaskPriority,
				dueDate: dueDate || null,
				endDate: endDate || null
			});

			for (const att of pendingAttachments) {
				await createAttachment({
					taskId: task.id,
					fileName: att.fileName,
					fileData: att.fileData,
					mimeType: att.mimeType,
					fileSize: att.fileSize
				});
				void logActivity(task.id, 'attachment_added', undefined, undefined, att.fileName);
			}

			await setTaskLabels(task.id, selectedLabelIds);

			// reload attachments from disk after save
			pendingAttachments = [];
			await loadAttachments(task.id);

			if (updated) {
				updated.labelIds = selectedLabelIds;
				onUpdated?.(updated);
			}
			open = false;
		} catch {
			error = 'Failed to update task';
		} finally {
			submitting = false;
		}
	}

	function formatDate(value: string) {
		return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(value));
	}

	function formatRelativeTime(value: string): string {
		const now = Date.now();
		const then = new Date(value).getTime();
		const diffMs = now - then;
		const diffSec = Math.round(diffMs / 1000);
		const diffMin = Math.round(diffSec / 60);
		const diffHr = Math.round(diffMin / 60);
		const diffDay = Math.round(diffHr / 24);

		if (diffSec < 60) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		if (diffHr < 24) return `${diffHr}h ago`;
		if (diffDay < 7) return `${diffDay}d ago`;
		return formatDate(value);
	}

	function activityMessage(entry: ActivityLog): string {
		switch (entry.action) {
			case 'created':
				return 'created this task';
			case 'status_changed':
				return `changed status${entry.oldValue ? ` from ${entry.oldValue}` : ''} to ${entry.newValue}`;
			case 'priority_changed':
				return `set priority to ${entry.newValue}`;
			case 'title_changed':
				return 'updated the title';
			case 'description_changed':
				return 'updated the description';
			case 'due_date_changed':
				return entry.newValue ? `set due date to ${entry.newValue}` : 'removed the due date';
			case 'end_date_changed':
				return entry.newValue ? `set end date to ${entry.newValue}` : 'removed the end date';
			case 'label_added':
				return `added label ${entry.newValue}`;
			case 'label_removed':
				return `removed label ${entry.oldValue}`;
			case 'attachment_added':
				return `attached ${entry.newValue}`;
			case 'attachment_removed':
				return 'removed an attachment';
			case 'subtask_added':
				return `added subtask "${entry.newValue}"`;
			case 'subtask_completed':
				return `completed subtask "${entry.newValue}"`;
			case 'subtask_uncompleted':
				return `reopened subtask "${entry.newValue}"`;
			case 'subtask_removed':
				return 'removed a subtask';
			case 'trashed':
				return 'moved this task to trash';
			case 'restored':
				return 'restored this task from trash';
			default:
				return String(entry.action).replace(/_/g, ' ');
		}
	}
</script>

<!-- backdrop + panel -->
{#if open}
	<div
		class="fixed inset-0 z-40 bg-background/40 backdrop-blur-[2px]"
		transition:fade={{ duration: 200 }}
		onclick={close}
		onkeydown={(e) => {
			if (e.key === 'Escape') close();
		}}
		role="button"
		tabindex="-1"
		aria-label="Close panel"
	></div>

	<div
		bind:this={panelRef}
		class="fixed top-0 right-0 z-50 h-screen w-full max-w-[640px] bg-card shadow-2xl"
		transition:fly={{ x: 640, duration: 280, opacity: 1 }}
		role="dialog"
		aria-modal="true"
		aria-label="Task detail"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape') close();
		}}
	>
		{#if task}
			<form
				onsubmit={(e) => {
					e.preventDefault();
					void handleSubmit();
				}}
				class="flex h-full flex-col"
			>
				<!-- header -->
				<div class="flex items-center justify-between px-5 pt-4 pb-3">
					<div class="flex items-center gap-2">
						<div
							class="flex size-5 items-center justify-center rounded-[5px] bg-primary text-[10px] leading-none font-bold text-primary-foreground"
						>
							{prefix.charAt(0)}
						</div>
						<span class="font-mono text-[12px] text-muted-foreground">
							{prefix}-{task.number}
						</span>
					</div>
					<div class="flex items-center gap-1">
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-sm"
										class={task.pinned
											? 'text-foreground'
											: 'text-muted-foreground hover:text-foreground'}
										aria-label={task.pinned ? 'Unpin task' : 'Pin task'}
										onclick={() => void handleTogglePin()}
									>
										<svg
											xmlns="http://www.w3.org/2000/svg"
											width="15"
											height="15"
											viewBox="0 0 24 24"
										>
											<title>pin_fill</title>
											<g id="pin_fill" fill="none">
												<path
													d="M24 0v24H0V0zM12.593 23.258l-.011.002-.071.035-.02.004-.014-.004-.071-.035c-.01-.004-.019-.001-.024.005l-.004.01-.017.428.005.02.01.013.104.074.015.004.012-.004.104-.074.012-.016.004-.017-.017-.427c-.002-.01-.009-.017-.017-.018m.265-.113-.013.002-.185.093-.01.01-.003.011.018.43.005.012.008.007.201.093c.012.004.023 0 .029-.008l.004-.014-.034-.614c-.003-.012-.01-.02-.02-.022m-.715.002a.023.023 0 0 0-.027.006l-.006.014-.034.614c0 .012.007.02.017.024l.015-.002.201-.093.01-.008.004-.011.017-.43-.003-.012-.01-.01z"
												/>
												<path
													fill="currentColor"
													d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
												/>
											</g>
										</svg>
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="bottom">{task.pinned ? 'Unpin' : 'Pin'}</Tooltip.Content>
						</Tooltip.Root>
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-sm"
										class="text-muted-foreground hover:text-destructive"
										aria-label="Delete task"
										onclick={() => {
											window.dispatchEvent(
												new CustomEvent('delete-task-from-panel', { detail: task.id })
											);
											close();
										}}
									>
										<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
											><path
												fill="currentColor"
												d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
											/></svg
										>
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="bottom">Delete task</Tooltip.Content>
						</Tooltip.Root>
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-sm"
										class="text-muted-foreground hover:text-foreground"
										aria-label="Close panel"
										onclick={close}
									>
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
											><path
												fill="currentColor"
												d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
											/></svg
										>
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="bottom">Close</Tooltip.Content>
						</Tooltip.Root>
					</div>
				</div>

				<!-- scrollable content -->
				<div class="flex flex-1 overflow-hidden">
					<!-- main content -->
					<div class="flex flex-1 flex-col overflow-y-auto px-5">
						<!-- title -->
						<Input
							bind:ref={titleRef}
							bind:value={title}
							placeholder="Task title"
							class="h-auto border-none bg-transparent px-3 py-1 text-[18px] font-semibold shadow-none placeholder:text-muted-foreground/40"
						/>

						<!-- description -->
						<div class="flex flex-col gap-1.5 pt-2">
							<div class="flex items-center justify-between">
								{#if description.trim()}
									<Button
										variant="ghost"
										class="flex h-auto items-center gap-1 p-0 text-[11px] text-muted-foreground/60 transition-colors hover:text-foreground"
										onclick={() => (previewMode = !previewMode)}
									>
										{#if previewMode}
											<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
												><path
													fill="currentColor"
													d="M7.06 16.836a1.25 1.25 0 0 1 1.86 1.666l-.091.102-2.298 2.298a1.5 1.5 0 0 1-2.008.103l-.114-.103-1.237-1.238a1.25 1.25 0 0 1 1.666-1.859l.102.091.53.53zM20 17.5a1.5 1.5 0 0 1 0 3h-8a1.5 1.5 0 1 1 0-3zM8.83 9.836a1.25 1.25 0 0 1 0 1.768l-2.3 2.298a1.5 1.5 0 0 1-2.122 0l-1.237-1.238a1.25 1.25 0 1 1 1.768-1.768l.53.53 1.59-1.59a1.25 1.25 0 0 1 1.769 0ZM20 10.5a1.5 1.5 0 0 1 .145 2.993L20 13.5h-8a1.5 1.5 0 0 1-.144-2.993L12 10.5zM7.06 2.836a1.25 1.25 0 0 1 1.86 1.666l-.091.101L6.53 6.902a1.5 1.5 0 0 1-2.008.103l-.114-.103-1.237-1.238a1.25 1.25 0 0 1 1.666-1.859l.102.091.53.53zM20 3.5a1.5 1.5 0 0 1 .145 2.993L20 6.5h-8a1.5 1.5 0 0 1-.144-2.993L12 3.5z"
												/></svg
											>
											<span>Edit</span>
										{:else}
											<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
												><path
													fill="currentColor"
													d="M12 5c3.679 0 8.162 2.417 9.73 5.901.146.328.27.71.27 1.099 0 .388-.123.771-.27 1.099C20.161 16.583 15.678 19 12 19c-3.679 0-8.162-2.417-9.73-5.901C2.124 12.77 2 12.389 2 12c0-.388.123-.771.27-1.099C3.839 7.417 8.322 5 12 5m0 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8m0 2a2 2 0 1 1 0 4 2 2 0 0 1 0-4"
												/></svg
											>
											<span>Preview</span>
										{/if}
									</Button>
								{:else}
									<span></span>
								{/if}
							</div>
							{#if previewMode}
								<div class="min-h-[60px] rounded-lg border border-border bg-muted/20 p-3">
									{#if description.trim()}
										<MarkdownRenderer content={description} />
									{:else}
										<span class="text-[12px] text-muted-foreground/50">Nothing to preview</span>
									{/if}
								</div>
							{:else}
								<Textarea
									bind:value={description}
									placeholder="Add a description... **markdown supported**"
									rows={4}
									class="min-h-[80px] border-none bg-transparent px-3 py-1 text-[13px] shadow-none placeholder:text-muted-foreground/50"
								/>
							{/if}
						</div>

						<!-- subtasks / checklist -->
						<div class="pt-4">
							<div class="flex items-center gap-2 pb-2">
								<svg
									class="text-muted-foreground/60"
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									><path
										fill="currentColor"
										d="M7 13a2 2 0 0 1 1.995 1.85L9 15v3a2 2 0 0 1-1.85 1.995L7 20H4a2 2 0 0 1-1.995-1.85L2 18v-3a2 2 0 0 1 1.85-1.995L4 13zm9 4a1 1 0 0 1 .117 1.993L16 19h-4a1 1 0 0 1-.117-1.993L12 17zm4-4a1 1 0 1 1 0 2h-8a1 1 0 1 1 0-2zM7 3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm9 4a1 1 0 0 1 .117 1.993L16 9h-4a1 1 0 0 1-.117-1.993L12 7zm4-4a1 1 0 0 1 .117 1.993L20 5h-8a1 1 0 0 1-.117-1.993L12 3z"
									/></svg
								>
								<span class="text-[12px] font-medium text-foreground">Subtasks</span>
								{#if subtasks.length > 0}
									<span class="text-[11px] text-muted-foreground/50"
										>{subtasks.filter((s) => s.completed).length}/{subtasks.length}</span
									>
									<div class="flex-1"></div>
									<div class="flex items-center gap-1.5">
										<span class="text-[11px] text-muted-foreground/50">{subtaskProgress}%</span>
										<Progress value={subtaskProgress} class="h-1 w-20" />
									</div>
								{/if}
							</div>

							{#if loadingSubtasks}
								<div class="flex items-center gap-2 py-2 text-[12px] text-muted-foreground">
									<Spinner class="size-3" />
									<span>Loading subtasks...</span>
								</div>
							{:else}
								{#if subtasks.length > 0}
									<div class="flex flex-col gap-0.5">
										{#each subtasks as subtask (subtask.id)}
											<div
												role="listitem"
												use:sortableItem={{
													dragData: subtask,
													container: 'subtasks',
													onDrop: (state: DragDropState<Subtask>) =>
														void handleSubtaskDrop(state, subtask)
												}}
												class="group/subtask flex cursor-grab items-center gap-2 rounded-md py-1 pr-1 transition-colors hover:bg-muted/30 active:cursor-grabbing"
											>
												<Checkbox
													checked={subtask.completed}
													onCheckedChange={() => void handleToggleSubtask(subtask)}
													class="size-4 shrink-0"
													aria-label={subtask.completed ? 'Mark as incomplete' : 'Mark as complete'}
												/>
												{#if editingSubtaskId === subtask.id}
													<Input
														bind:value={editingSubtaskTitle}
														onkeydown={(e) => {
															if (e.key === 'Enter') {
																e.preventDefault();
																void handleRenameSubtask(subtask.id);
															}
															if (e.key === 'Escape') cancelEditSubtask();
														}}
														onblur={() => void handleRenameSubtask(subtask.id)}
														class="h-5 flex-1 rounded border-none bg-transparent px-2 text-[13px] text-foreground outline-none"
														aria-label="Rename subtask"
													/>
												{:else}
													<span
														class="flex-1 cursor-text text-[13px] {subtask.completed
															? 'text-muted-foreground/50 line-through'
															: 'text-foreground/90'}"
														ondblclick={() => startEditSubtask(subtask)}
														role="button"
														tabindex="0">{subtask.title}</span
													>
												{/if}
												{#if editingSubtaskId !== subtask.id}
													<Button
														variant="ghost"
														size="icon-xs"
														class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/30 opacity-0 transition-all group-hover/subtask:opacity-100 hover:text-foreground"
														onclick={() => startEditSubtask(subtask)}
														aria-label="Rename subtask"
													>
														<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
															><path
																fill="currentColor"
																d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
															/></svg
														>
													</Button>
													<Button
														variant="ghost"
														size="icon-xs"
														class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/30 opacity-0 transition-all group-hover/subtask:opacity-100 hover:text-destructive"
														onclick={() => void handleRemoveSubtask(subtask.id)}
														aria-label="Remove subtask"
													>
														<svg width="12" height="12" viewBox="0 0 24 24" fill="none"
															><path
																fill="currentColor"
																d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
															/></svg
														>
													</Button>
												{/if}
											</div>
										{/each}
									</div>
								{/if}

								<!-- add subtask -->
								<div class="flex items-center gap-2 pt-1">
									<svg
										class="text-muted-foreground/30"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
										/></svg
									>
									<Input
										bind:value={newSubtaskTitle}
										placeholder="Add a subtask..."
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.preventDefault();
												void handleAddSubtask();
											}
										}}
										class="h-7 flex-1 border-none bg-transparent px-2 text-[13px] text-foreground outline-none placeholder:text-muted-foreground/40"
									/>
									{#if newSubtaskTitle.trim()}
										<Button
											variant="ghost"
											size="sm"
											class="h-6 px-2 text-[11px]"
											onclick={() => void handleAddSubtask()}
										>
											Add
										</Button>
									{/if}
								</div>
							{/if}
						</div>

						<!-- attachments -->
						{#if loadingAttachments}
							<div class="flex items-center gap-2 pt-3 text-[12px] text-muted-foreground">
								<Spinner class="size-3" />
								<span>Loading attachments...</span>
							</div>
						{:else if attachments.length > 0 || pendingAttachments.length > 0}
							<div class="pt-3">
								<div
									class="flex items-center gap-1.5 pb-2 text-[11px] font-medium text-muted-foreground/60"
								>
									<svg width="12" height="12" viewBox="0 0 24 24" fill="none"
										><path
											fill="currentColor"
											d="M15.889 9.525a1.5 1.5 0 0 1 2.007-.103l.114.103 2.122 2.121a6 6 0 0 1-8.303 8.661l-.183-.175-2.121-2.122a1.5 1.5 0 0 1 2.007-2.224l.114.103 2.122 2.121a3 3 0 0 0 4.377-4.098l-.135-.144-2.121-2.122a1.5 1.5 0 0 1 0-2.121m-7.071-.707a1.5 1.5 0 0 1 2.007-.103l.114.103 4.243 4.243a1.5 1.5 0 0 1-2.008 2.224l-.114-.103-4.242-4.243a1.5 1.5 0 0 1 0-2.121m-4.95-4.95a6 6 0 0 1 8.302-.175l.184.175 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.114-.103-2.12-2.121a3 3 0 0 0-4.378 4.098l.135.144 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.113-.103-2.122-2.121a6 6 0 0 1 0-8.486"
										/></svg
									>
									<span>Attachments</span>
								</div>
								<div class="flex flex-wrap gap-2">
									{#each attachments as att (att.id)}
										<div
											class="group/att relative size-20 cursor-zoom-in overflow-hidden rounded-lg border border-border bg-muted/30"
											role="button"
											tabindex="0"
											onclick={() => {
												const url = attachmentUrls[att.id];
												if (url) lightboxUrl = url;
											}}
											onkeydown={(e) => {
												const url = attachmentUrls[att.id];
												if (e.key === 'Enter' && url) lightboxUrl = url;
											}}
										>
											{#if isImage(att.mimeType)}
												<img
													src={attachmentUrls[att.id] ?? ''}
													alt={att.fileName}
													class="size-full object-cover"
												/>
											{:else}
												<div
													class="flex size-full flex-col items-center justify-center gap-1 p-1 text-center"
												>
													<svg
														class="text-muted-foreground"
														width="20"
														height="20"
														viewBox="0 0 24 24"
														fill="none"
														><path
															fill="currentColor"
															d="M4.5 15V9a1.5 1.5 0 1 1 3 0v6a4.5 4.5 0 1 0 9 0V7a2.5 2.5 0 0 0-5 0v8a.5.5 0 0 0 1 0V9a1.5 1.5 0 0 1 3 0v6a3.5 3.5 0 1 1-7 0V7a5.5 5.5 0 1 1 11 0v8a7.5 7.5 0 0 1-15 0"
														/></svg
													>
													<span class="line-clamp-1 text-[9px] text-muted-foreground"
														>{att.fileName}</span
													>
												</div>
											{/if}
											<Button
												variant="ghost"
												size="icon-xs"
												class="absolute top-1 right-1 flex size-4 items-center justify-center rounded bg-background/80 text-foreground opacity-0 transition-opacity group-hover/att:opacity-100"
												onclick={() => void deleteExisting(att.id)}
												aria-label="Remove attachment"
											>
												<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
													/></svg
												>
											</Button>
											<Button
												variant="ghost"
												size="icon-xs"
												class="absolute bottom-1 left-1 flex size-4 items-center justify-center rounded bg-background/80 text-foreground opacity-0 transition-opacity group-hover/att:opacity-100"
												onclick={() => void handleDownloadAttachment(att)}
												aria-label="Download attachment"
											>
												<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="M12 2a1 1 0 0 1 1 1v10.586l3.293-3.293a1 1 0 0 1 1.414 1.414l-5 5a1 1 0 0 1-1.414 0l-5-5a1 1 0 0 1 1.414-1.414L11 13.586V3a1 1 0 0 1 1-1M5 19a1 1 0 0 1 1-1h12a1 1 0 1 1 0 2H6a1 1 0 0 1-1-1"
													/></svg
												>
											</Button>
										</div>
									{/each}
									{#each pendingAttachments as att, i (i)}
										<div
											class="group/pending relative size-20 cursor-zoom-in overflow-hidden rounded-lg border border-dashed border-border bg-muted/30"
											role="button"
											tabindex="0"
											onclick={() => {
												if (att.fileData) lightboxUrl = att.fileData;
											}}
											onkeydown={(e) => {
												if (e.key === 'Enter' && att.fileData) lightboxUrl = att.fileData;
											}}
										>
											{#if isImage(att.mimeType)}
												<img src={att.fileData} alt={att.fileName} class="size-full object-cover" />
											{:else}
												<div
													class="flex size-full flex-col items-center justify-center gap-1 p-1 text-center"
												>
													<svg
														class="text-muted-foreground"
														width="20"
														height="20"
														viewBox="0 0 24 24"
														fill="none"
														><path
															fill="currentColor"
															d="M4.5 15V9a1.5 1.5 0 1 1 3 0v6a4.5 4.5 0 1 0 9 0V7a2.5 2.5 0 0 0-5 0v8a.5.5 0 0 0 1 0V9a1.5 1.5 0 0 1 3 0v6a3.5 3.5 0 1 1-7 0V7a5.5 5.5 0 1 1 11 0v8a7.5 7.5 0 0 1-15 0"
														/></svg
													>
													<span class="line-clamp-1 text-[9px] text-muted-foreground"
														>{att.fileName}</span
													>
												</div>
											{/if}
											<Button
												variant="ghost"
												size="icon-xs"
												class="absolute top-1 right-1 flex size-4 items-center justify-center rounded bg-background/80 text-foreground opacity-0 transition-opacity group-hover/pending:opacity-100"
												onclick={() => removePending(i)}
												aria-label="Remove attachment"
											>
												<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
													/></svg
												>
											</Button>
										</div>
									{/each}
								</div>
							</div>
						{/if}

						{#if error}
							<p class="pt-3 text-[12px] text-destructive" role="alert">{error}</p>
						{/if}

						<!-- activity history -->
						{#if activities.length > 0}
							<div class="pt-5">
								<div
									class="flex items-center gap-1.5 pb-2 text-[11px] font-medium text-muted-foreground/60"
								>
									<svg width="12" height="12" viewBox="0 0 24 24" fill="none">
										<path
											fill="none"
											stroke="currentColor"
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M12 6v6l4 2M12 2a10 10 0 1 0 0 20a10 10 0 0 0 0-20"
										/>
									</svg>
									<span>Activity</span>
								</div>
								<div class="flex flex-col">
									{#each activities as entry, i (entry.id)}
										<div class="flex items-start gap-2.5">
											<!-- timeline dot + line -->
											<div class="flex flex-col items-center pt-1">
												<div class="size-1.5 rounded-full bg-muted-foreground/30"></div>
												{#if i < activities.length - 1}
													<div class="min-h-[20px] w-px flex-1 bg-border/50"></div>
												{/if}
											</div>
											<!-- content -->
											<div class="flex-1 pb-3">
												<div class="flex items-center gap-2">
													<p class="text-[12px] text-muted-foreground/80">
														<span class="text-foreground/90">{activityMessage(entry)}</span>
													</p>
													{#if entry.source === 'cli'}
														<Badge
															variant="outline"
															class="h-4 gap-0.5 border-muted-foreground/20 px-1.5 text-[10px] font-medium text-muted-foreground/60"
														>
															<svg width="9" height="9" viewBox="0 0 24 24" fill="none">
																<path
																	fill="none"
																	stroke="currentColor"
																	stroke-linecap="round"
																	stroke-linejoin="round"
																	stroke-width="2.5"
																	d="M7 9l-3 3 3 3M17 9l3 3-3 3M14 5l-4 14"
																/>
															</svg>
															CLI
														</Badge>
													{/if}
												</div>
												<span class="text-[11px] text-muted-foreground/40"
													>{formatRelativeTime(entry.createdAt)}</span
												>
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/if}

						<!-- timestamps -->
						<div class="flex items-center gap-3 pt-2 pb-4 text-[11px] text-muted-foreground/40">
							<span>Created {formatDate(task.createdAt)}</span>
							<span class="size-1 rounded-full bg-muted-foreground/20"></span>
							<span>Updated {formatDate(task.updatedAt)}</span>
						</div>
					</div>

					<!-- properties sidebar -->
					<aside class="flex w-[200px] shrink-0 flex-col gap-px overflow-y-auto bg-muted/10 p-3">
						<!-- status -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Status</span>
							<Popover.Root>
								<Popover.Trigger>
									{#snippet child({ props })}
										<Button
											{...props}
											variant="ghost"
											class="flex h-auto w-full items-center justify-start gap-2 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-muted/50"
										>
											<StatusIcon {status} size={14} />
											<span>{statusLabels[status]}</span>
										</Button>
									{/snippet}
								</Popover.Trigger>
								<Popover.Content class="w-44 p-1.5" align="start">
									<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
										Change status
									</div>
									{#each statusOrder as s (s)}
										<Button
											variant="ghost"
											class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
											onclick={() => (status = s)}
										>
											<StatusIcon status={s} size={14} />
											<span>{statusLabels[s]}</span>
											{#if status === s}
												<svg
													class="ml-auto text-muted-foreground"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
													><path
														fill="currentColor"
														d="M13.06 16.06a1.5 1.5 0 0 1-2.12 0l-5.658-5.656a1.5 1.5 0 1 1 2.122-2.121L12 12.879l4.596-4.596a1.5 1.5 0 0 1 2.122 2.12l-5.657 5.658Z"
													/></svg
												>
											{/if}
										</Button>
									{/each}
								</Popover.Content>
							</Popover.Root>
						</div>

						<Separator />

						<!-- priority -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Priority</span>
							<Popover.Root>
								<Popover.Trigger>
									{#snippet child({ props })}
										<Button
											{...props}
											variant="ghost"
											class="flex h-auto w-full items-center justify-start gap-2 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-muted/50"
										>
											{#if Number(priority) === 1}
												<svg
													class="shrink-0 text-orange-500"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
												>
													<path
														fill="currentColor"
														d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
													/>
												</svg>
											{:else}
												<svg
													class="shrink-0"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
												>
													<rect
														x="3"
														y="14"
														width="3.5"
														height="7"
														rx="1"
														fill="currentColor"
														opacity={Number(priority) >= 2 ? 1 : 0.25}
													/>
													<rect
														x="10.25"
														y="9"
														width="3.5"
														height="12"
														rx="1"
														fill="currentColor"
														opacity={Number(priority) >= 3 ? 1 : 0.25}
													/>
													<rect
														x="17.5"
														y="4"
														width="3.5"
														height="17"
														rx="1"
														fill="currentColor"
														opacity={Number(priority) >= 4 ? 1 : 0.25}
													/>
												</svg>
											{/if}
											<span>{priorityLabels[priority]}</span>
										</Button>
									{/snippet}
								</Popover.Trigger>
								<Popover.Content class="w-48 p-1.5" align="start">
									<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
										Set priority
									</div>
									{#each [0, 1, 2, 3, 4] as p (p)}
										<Button
											variant="ghost"
											class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
											onclick={() => (priority = String(p))}
										>
											{#if p === 1}
												<svg
													class="shrink-0 text-orange-500"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
												>
													<path
														fill="currentColor"
														d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
													/>
												</svg>
											{:else}
												<svg
													class="shrink-0"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
												>
													<rect
														x="3"
														y="14"
														width="3.5"
														height="7"
														rx="1"
														fill="currentColor"
														opacity={p >= 2 ? 1 : 0.25}
													/>
													<rect
														x="10.25"
														y="9"
														width="3.5"
														height="12"
														rx="1"
														fill="currentColor"
														opacity={p >= 3 ? 1 : 0.25}
													/>
													<rect
														x="17.5"
														y="4"
														width="3.5"
														height="17"
														rx="1"
														fill="currentColor"
														opacity={p >= 4 ? 1 : 0.25}
													/>
												</svg>
											{/if}
											<span>{priorityConfig[p].label}</span>
											{#if Number(priority) === p}
												<svg
													class="ml-auto text-muted-foreground"
													width="14"
													height="14"
													viewBox="0 0 24 24"
													fill="none"
													><path
														fill="currentColor"
														d="M13.06 16.06a1.5 1.5 0 0 1-2.12 0l-5.658-5.656a1.5 1.5 0 1 1 2.122-2.121L12 12.879l4.596-4.596a1.5 1.5 0 0 1 2.122 2.12l-5.657 5.658Z"
													/></svg
												>
											{/if}
										</Button>
									{/each}
								</Popover.Content>
							</Popover.Root>
						</div>

						<Separator />

						<!-- due date -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Due date</span>
							<DueDatePicker
								value={dueDate}
								onSelect={(d) => (dueDate = d)}
								onClear={() => (dueDate = '')}
							/>
						</div>

						<!-- end date -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">End date</span>
							<DueDatePicker
								title="End date"
								value={endDate}
								onSelect={(d) => (endDate = d)}
								onClear={() => (endDate = '')}
							/>
						</div>

						<Separator />

						<!-- labels -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Labels</span>
							<LabelSelector
								bind:selectedIds={selectedLabelIds}
								{labels}
								onCreated={onLabelCreated}
								onUpdated={onLabelUpdated}
								onRemoved={onLabelRemoved}
							/>
						</div>
					</aside>
				</div>

				<!-- footer -->
				<Input
					bind:ref={fileInput}
					type="file"
					accept="image/*"
					multiple
					class="hidden"
					onchange={handleFileSelect}
				/>
				<div class="flex items-center justify-between border-t border-border px-5 py-3">
					<div class="flex items-center gap-2">
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-sm"
										class="text-muted-foreground"
										aria-label="Attach file"
										onclick={() => fileInput?.click()}
									>
										{#if uploading}
											<Spinner class="size-4" />
										{:else}
											<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
												><path
													fill="currentColor"
													d="M15.889 9.525a1.5 1.5 0 0 1 2.007-.103l.114.103 2.122 2.121a6 6 0 0 1-8.303 8.661l-.183-.175-2.121-2.122a1.5 1.5 0 0 1 2.007-2.224l.114.103 2.122 2.121a3 3 0 0 0 4.377-4.098l-.135-.144-2.121-2.122a1.5 1.5 0 0 1 0-2.121m-7.071-.707a1.5 1.5 0 0 1 2.007-.103l.114.103 4.243 4.243a1.5 1.5 0 0 1-2.008 2.224l-.114-.103-4.242-4.243a1.5 1.5 0 0 1 0-2.121m-4.95-4.95a6 6 0 0 1 8.302-.175l.184.175 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.114-.103-2.12-2.121a3 3 0 0 0-4.378 4.098l.135.144 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.113-.103-2.122-2.121a6 6 0 0 1 0-8.486"
												/></svg
											>
										{/if}
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="top">Attach file</Tooltip.Content>
						</Tooltip.Root>
					</div>
					<div class="flex items-center gap-2">
						<Button variant="ghost" size="sm" onclick={close}>Cancel</Button>
						<Button type="submit" size="sm" disabled={submitting}>
							{submitting ? 'Saving...' : 'Save changes'}
						</Button>
					</div>
				</div>
			</form>
		{/if}
	</div>

	{#if lightboxUrl}
		<div
			bind:this={lightboxRef}
			class="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 p-8"
			onclick={() => (lightboxUrl = null)}
			onpointerdowncapture={(e) => {
				e.stopPropagation();
				lightboxUrl = null;
			}}
			onkeydown={(e) => {
				if (e.key === 'Escape') {
					lightboxUrl = null;
					e.stopPropagation();
				}
			}}
			role="button"
			tabindex="-1"
			transition:fade={{ duration: 150 }}
		>
			<img
				src={lightboxUrl}
				alt="preview"
				class="max-h-full max-w-full rounded-lg object-contain shadow-2xl"
				transition:scale={{ duration: 150, start: 0.95 }}
			/>
			<button
				class="absolute top-4 right-4 flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
				onclick={() => (lightboxUrl = null)}
				aria-label="Close preview"
			>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
					/></svg
				>
			</button>
		</div>
	{/if}
{/if}
