<script lang="ts">
	import { onMount } from 'svelte';
	import { update, togglePin, findById } from '$lib/repositories/task.repository';
	import PriorityIcon from './PriorityIcon.svelte';
	import PriorityMenu from './PriorityMenu.svelte';
	import StatusMenu from './StatusMenu.svelte';
	import Lightbox from './Lightbox.svelte';
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
	import { sortableItem, reorderArray, type DragDropState } from '$lib/dnd';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { isTauri } from '$lib/db/client';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import { Progress } from '$lib/components/ui/progress/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import LabelSelector from './LabelSelector.svelte';
	import StatusIcon from './StatusIcon.svelte';
	import MarkdownRenderer from '../MarkdownRenderer.svelte';
	import DueDatePicker from './DueDatePicker.svelte';
	import RecurrencePicker from './RecurrencePicker.svelte';
	import ReminderPicker from './ReminderPicker.svelte';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import Icon from '$lib/components/Icon.svelte';

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

	// the parent unmounts this panel as soon as `open` flips false, which would
	// cut off the sheet exit animation; keep our own open state and only release
	// the parent once the close animation has finished
	let sheetOpen = $state(open);

	$effect(() => {
		if (open) sheetOpen = true;
	});

	function handleSheetOpenChangeComplete(isOpen: boolean) {
		if (!isOpen) open = false;
	}

	let title = $state('');
	let description = $state('');
	let status = $state<TaskStatus>('todo');
	let priority = $state('0');
	let dueDate = $state<string>('');
	let endDate = $state<string>('');
	let reminderAt = $state<string>('');
	let recurrenceRule = $state<string>('');
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let previewMode = $state(false);
	let titleRef = $state<HTMLInputElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let attachments = $state<TaskAttachment[]>([]);
	let attachmentUrls = $state<Record<string, string>>({});
	let lightboxUrl = $state<string | null>(null);
	let lightboxAtt = $state<TaskAttachment | null>(null);
	let lightboxPending: { fileName: string; fileData: string } | null = $state(null);
	let pendingAttachments = $state<
		{ fileName: string; fileData: string; mimeType: string; fileSize: number }[]
	>([]);
	let uploading = $state(false);
	let loadingAttachments = $state(false);
	let selectedLabelIds = $state<string[]>([]);
	// subtasks
	let subtasks = $state<Subtask[]>([]);
	let newSubtaskTitle = $state('');
	let loadingSubtasks = $state(false);
	let editingSubtaskId = $state<string | null>(null);
	let editingSubtaskTitle = $state('');

	// field snapshot taken when the panel opens; save stays disabled until
	// something actually changes
	let original = $state<{
		title: string;
		description: string;
		status: TaskStatus;
		priority: string;
		dueDate: string;
		endDate: string;
		reminderAt: string;
		recurrenceRule: string;
		labelIds: string[];
		pendingCount: number;
	} | null>(null);
	let isDirty = $derived(
		!!original &&
			(original.title !== title ||
				original.description !== description ||
				original.status !== status ||
				original.priority !== priority ||
				original.dueDate !== dueDate ||
				original.endDate !== endDate ||
				original.reminderAt !== reminderAt ||
				original.recurrenceRule !== recurrenceRule ||
				original.pendingCount !== pendingAttachments.length ||
				original.labelIds.length !== selectedLabelIds.length ||
				selectedLabelIds.some((id) => !original!.labelIds.includes(id)))
	);

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
		reminderAt = task.reminderAt ?? '';
		recurrenceRule = task.recurrenceRule ?? '';
		error = null;
		previewMode = false;
		pendingAttachments = [];
		attachments = [];
		attachmentUrls = {};
		selectedLabelIds = [];
		newSubtaskTitle = '';
		original = {
			title: task.title,
			description: task.description ?? '',
			status: task.status,
			priority: String(task.priority),
			dueDate: task.dueDate ?? '',
			endDate: task.endDate ?? '',
			reminderAt: task.reminderAt ?? '',
			recurrenceRule: task.recurrenceRule ?? '',
			labelIds: [],
			pendingCount: 0
		};
		void loadAttachments(task.id);
		void loadTaskLabels(task.id);
		void loadSubtasks(task.id);
		void loadActivity(task.id);
		// the list view omits the description column; fetch the full task so
		// existing descriptions show up
		void loadFullTask(task.id);
	});

	async function loadFullTask(id: string) {
		const full = await findById(id).catch(() => null);
		// the open effect already set the fields; only patch the heavy fields
		// the list doesn't carry, and only while the same task is still open
		if (!full || full.id !== task?.id) return;
		if (!description) {
			description = full.description ?? '';
			if (original && !original.description) original.description = full.description ?? '';
		}
	}

	$effect(() => {
		// keep focus inside the panel, also return it after the lightbox closes
		if (open && !lightboxUrl) requestAnimationFrame(() => titleRef?.focus());
	});

	async function loadTaskLabels(taskId: string) {
		try {
			selectedLabelIds = await findLabelIdsByTaskId(taskId);
		} catch {
			selectedLabelIds = [];
		}
		// labels loaded after the snapshot: they are the original state, not a change
		if (original) original.labelIds = [...selectedLabelIds];
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

	// pending attachments are in-memory data urls: a plain anchor download saves them
	function downloadDataUrl(dataUrl: string, fileName: string) {
		const a = document.createElement('a');
		a.href = dataUrl;
		a.download = fileName;
		a.click();
		a.remove();
	}

	async function handleDownloadAttachment(att: TaskAttachment) {
		try {
			if (!isTauri()) {
				await downloadAttachment(att.id, att.fileName);
				return;
			}
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
			await downloadAttachment(att.id, att.fileName, destPath);
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
		sheetOpen = false;
	}

	onMount(() => {
		const registry = getShortcutRegistry();

		const unregisterClose = registry.register({
			id: 'close',
			enabled: () => open,
			allowInInput: true,
			run: () => close()
		});

		const unregisterSave = registry.register({
			id: 'save-task',
			enabled: () => open,
			allowInInput: true,
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
				endDate: endDate || null,
				reminderAt: reminderAt || null,
				recurrenceRule: recurrenceRule || null
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
			close();
		} catch {
			error = 'Failed to update task';
		} finally {
			submitting = false;
		}
	}

	function formatDate(value: string) {
		return new Intl.DateTimeFormat('en-US', { dateStyle: 'medium' }).format(new Date(value));
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
			case 'reminder_changed':
				return entry.newValue ? `set a reminder for ${entry.newValue}` : 'removed the reminder';
			case 'recurrence_changed':
				return entry.newValue
					? `set the task to repeat ${entry.newValue.toLowerCase()}`
					: 'removed the repeat schedule';
			case 'recurrence_spawned':
				return `scheduled the next occurrence for ${entry.newValue}`;
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
<Sheet.Root bind:open={sheetOpen} onOpenChangeComplete={handleSheetOpenChangeComplete}>
	<Sheet.Content
		side="right"
		showCloseButton={false}
		class="w-full! max-w-[640px]! gap-0 bg-card shadow-2xl sm:max-w-[680px]! lg:max-w-[720px]!"
		aria-label="Task detail"
	>
		{#if task}
			<Sheet.Title class="sr-only">Task detail</Sheet.Title>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					void handleSubmit();
				}}
				class="flex h-full flex-col"
			>
				<!-- header -->
				<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
					<div class="flex items-center gap-2">
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
										<Icon icon="pin" size={15} />
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
										<Icon icon="trash" size={15} />
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
										<Icon icon="close" size={16} />
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="bottom">Close</Tooltip.Content>
						</Tooltip.Root>
					</div>
				</div>

				<!-- scrollable content -->
				<div class="flex flex-1 flex-col overflow-hidden md:flex-row">
					<!-- main content -->
					<div class="flex flex-1 flex-col overflow-y-auto px-4 sm:px-5">
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
											<Icon icon="checklist" size={11} />
											<span>Edit</span>
										{:else}
											<Icon icon="eye" size={11} />
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
								<Icon icon="checklist" size={14} class="text-muted-foreground/60" />
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
														<Icon icon="pencil" size={11} />
													</Button>
													<Button
														variant="ghost"
														size="icon-xs"
														class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/30 opacity-0 transition-all group-hover/subtask:opacity-100 hover:text-destructive"
														onclick={() => void handleRemoveSubtask(subtask.id)}
														aria-label="Remove subtask"
													>
														<Icon icon="close" size={12} />
													</Button>
												{/if}
											</div>
										{/each}
									</div>
								{/if}

								<!-- add subtask -->
								<div class="flex items-center gap-2 pt-1">
									<Icon icon="add" size={14} class="text-muted-foreground/30" />
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
									<Icon icon="paperclip" size={12} />
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
												if (url) {
													lightboxUrl = url;
													lightboxAtt = att;
													lightboxPending = null;
												}
											}}
											onkeydown={(e) => {
												const url = attachmentUrls[att.id];
												if (e.key === 'Enter' && url) {
													lightboxUrl = url;
													lightboxAtt = att;
													lightboxPending = null;
												}
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
													<Icon icon="paperclip" size={20} class="text-muted-foreground" />
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
												<Icon icon="close" size={10} />
											</Button>
											<Button
												variant="ghost"
												size="icon-xs"
												class="absolute bottom-1 left-1 flex size-4 items-center justify-center rounded bg-background/80 text-foreground opacity-0 transition-opacity group-hover/att:opacity-100"
												onclick={() => void handleDownloadAttachment(att)}
												aria-label="Download attachment"
											>
												<Icon icon="download" size={10} />
											</Button>
										</div>
									{/each}
									{#each pendingAttachments as att, i (i)}
										<div
											class="group/pending relative size-20 cursor-zoom-in overflow-hidden rounded-lg border border-dashed border-border bg-muted/30"
											role="button"
											tabindex="0"
											onclick={() => {
												if (att.fileData) {
													lightboxUrl = att.fileData;
													lightboxPending = att;
													lightboxAtt = null;
												}
											}}
											onkeydown={(e) => {
												if (e.key === 'Enter' && att.fileData) {
													lightboxUrl = att.fileData;
													lightboxPending = att;
													lightboxAtt = null;
												}
											}}
										>
											{#if isImage(att.mimeType)}
												<img src={att.fileData} alt={att.fileName} class="size-full object-cover" />
											{:else}
												<div
													class="flex size-full flex-col items-center justify-center gap-1 p-1 text-center"
												>
													<Icon icon="paperclip" size={20} class="text-muted-foreground" />
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
												<Icon icon="close" size={10} />
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
									<Icon icon="clock" size={12} />
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
															<Icon icon="code" size={9} />
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
					<aside
						class="flex shrink-0 flex-col gap-px border-t border-border/60 bg-muted/10 p-3 md:w-[200px] md:overflow-y-auto md:border-t-0 md:border-l"
					>
						<!-- status -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Status</span>
							<StatusMenu value={status} onSelect={(s) => (status = s)}>
								{#snippet trigger(props)}
									<Button
										{...props}
										variant="ghost"
										class="flex h-auto w-full items-center justify-start gap-2 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-muted/50"
									>
										<StatusIcon {status} size={14} />
										<span>{statusLabels[status]}</span>
									</Button>
								{/snippet}
							</StatusMenu>
						</div>

						<Separator />

						<!-- priority -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Priority</span>
							<PriorityMenu value={Number(priority)} onSelect={(p) => (priority = String(p))}>
								{#snippet trigger(props)}
									<Button
										{...props}
										variant="ghost"
										class="flex h-auto w-full items-center justify-start gap-2 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-muted/50"
									>
										{#if Number(priority) > 0}
											<PriorityIcon priority={Number(priority)} size={14} />
										{/if}
										<span>{priorityLabels[priority]}</span>
									</Button>
								{/snippet}
							</PriorityMenu>
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

						<!-- repeat -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Repeat</span>
							<RecurrencePicker
								value={recurrenceRule}
								anchorDate={dueDate || null}
								onSelect={(rule) => (recurrenceRule = rule)}
								onClear={() => (recurrenceRule = '')}
							/>
						</div>

						<!-- reminder -->
						<div class="flex flex-col gap-1 py-1.5">
							<span class="text-[11px] font-medium text-muted-foreground/60">Reminder</span>
							<ReminderPicker
								value={reminderAt}
								onSelect={(iso) => (reminderAt = iso)}
								onClear={() => (reminderAt = '')}
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
				<div class="flex items-center justify-between border-t border-border px-4 py-3 sm:px-5">
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
											<Icon icon="paperclip" size={16} />
										{/if}
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="top">Attach file</Tooltip.Content>
						</Tooltip.Root>
					</div>
					<div class="flex items-center gap-2">
						<Button variant="ghost" size="sm" onclick={close}>Cancel</Button>
						<Button type="submit" size="sm" disabled={submitting || !isDirty}>
							{submitting ? 'Saving...' : 'Save changes'}
						</Button>
					</div>
				</div>
			</form>
		{/if}
	</Sheet.Content>
</Sheet.Root>

<Lightbox
	url={lightboxUrl}
	onClose={() => {
		lightboxUrl = null;
		lightboxAtt = null;
		lightboxPending = null;
	}}
	onDownload={() => {
		if (lightboxAtt) void handleDownloadAttachment(lightboxAtt);
		else if (lightboxPending) downloadDataUrl(lightboxPending.fileData, lightboxPending.fileName);
	}}
/>
