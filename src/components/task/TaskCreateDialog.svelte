<script lang="ts">
	import { create } from '$lib/repositories/task.repository';
	import { create as createAttachment } from '$lib/repositories/attachment.repository';
	import { setTaskLabels } from '$lib/repositories/label.repository';
	import type { Label } from '$lib/types/label';
	import type { Project } from '$lib/types/project';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import { Button } from '$lib/components/ui/button/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import StatusIcon from './StatusIcon.svelte';
	import Lightbox from './Lightbox.svelte';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';

	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Label as FormLabel } from '$lib/components/ui/label/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import LabelSelector from './LabelSelector.svelte';
	import MarkdownRenderer from '../MarkdownRenderer.svelte';
	import DueDatePicker from './DueDatePicker.svelte';
	import RecurrencePicker from './RecurrencePicker.svelte';
	import ReminderPicker from './ReminderPicker.svelte';
	import { getSettings } from '$lib/stores/settings';
	import { parseEstimate } from '$lib/task/time';
	import Icon from '$lib/components/Icon.svelte';

	type PendingAttachment = {
		fileName: string;
		fileData: string;
		mimeType: string;
		fileSize: number;
	};

	type Props = {
		open?: boolean;
		projects: Project[];
		labels: Label[];
		initialDueDate?: string | null;
		initialTitle?: string | null;
		initialParentId?: string | null;
		onCreated?: (task: Task) => void;
		onLabelCreated?: (label: Label) => void;
		onLabelUpdated?: (label: Label) => void;
		onLabelRemoved?: (id: string) => void;
	};

	let {
		open = $bindable(false),
		projects,
		labels,
		initialDueDate = null,
		initialTitle = null,
		initialParentId = null,
		onCreated,
		onLabelCreated,
		onLabelUpdated,
		onLabelRemoved
	}: Props = $props();
	let title = $state('');
	let description = $state('');
	const defaultSettings = getSettings();
	let status = $state<TaskStatus>(defaultSettings.defaultStatus);
	let priority = $state(String(defaultSettings.defaultPriority));
	let projectId = $state('');
	let dueDate = $state<string>('');
	let endDate = $state<string>('');
	let reminderAt = $state<string>('');
	let recurrenceRule = $state('');
	let estimateText = $state('');
	let selectedLabelIds = $state<string[]>([]);
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let createMore = $state(false);
	let titleRef = $state<HTMLInputElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let attachments = $state<PendingAttachment[]>([]);
	let uploading = $state(false);
	let previewMode = $state(false);
	let lightboxUrl = $state<string | null>(null);
	let lightboxAtt: PendingAttachment | null = $state(null);

	// attachments live as data urls before the task exists: a plain anchor download saves them
	function downloadDataUrl(dataUrl: string, fileName: string) {
		const a = document.createElement('a');
		a.href = dataUrl;
		a.download = fileName;
		a.click();
		a.remove();
	}

	const statusLabels: Record<TaskStatus, string> = {
		backlog: 'Backlog',
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

	$effect(() => {
		if (open && !projectId && projects.length > 0) projectId = projects[0].id;
	});

	$effect(() => {
		if (!open) return;
		dueDate = initialDueDate ?? '';
		title = initialTitle ?? '';
	});

	$effect(() => {
		if (open) requestAnimationFrame(() => titleRef?.focus());
	});

	// draft autosave: a dialog closed without submitting keeps its fields in
	// localStorage; an explicit opener (palette prefill, calendar "+", new
	// sub-task) starts a draftless session and clears any older draft
	const DRAFT_KEY = 'tack-create-task-draft';
	let draftEnabled = $state(false);
	let draftTimer: ReturnType<typeof setTimeout> | null = null;

	function clearDraft() {
		try {
			localStorage.removeItem(DRAFT_KEY);
		} catch {
			// localStorage unavailable
		}
	}

	$effect(() => {
		if (!open) {
			draftEnabled = false;
			return;
		}
		if (initialTitle || initialDueDate || initialParentId) {
			clearDraft();
			return;
		}
		try {
			const raw = localStorage.getItem(DRAFT_KEY);
			if (raw) {
				const d = JSON.parse(raw) as Record<string, never>;
				title = d.title ?? title;
				description = d.description ?? description;
				status = d.status ?? status;
				priority = d.priority ?? priority;
				projectId = d.projectId ?? projectId;
				dueDate = d.dueDate ?? dueDate;
				endDate = d.endDate ?? endDate;
				reminderAt = d.reminderAt ?? reminderAt;
				recurrenceRule = d.recurrenceRule ?? recurrenceRule;
				estimateText = d.estimateText ?? estimateText;
				selectedLabelIds = d.selectedLabelIds ?? selectedLabelIds;
			}
		} catch {
			// corrupt or missing draft: start empty
		}
		draftEnabled = true;
	});

	$effect(() => {
		if (!open || !draftEnabled) return;
		const snapshot = JSON.stringify({
			title,
			description,
			status,
			priority,
			projectId,
			dueDate,
			endDate,
			reminderAt,
			recurrenceRule,
			estimateText,
			selectedLabelIds
		});
		if (draftTimer) clearTimeout(draftTimer);
		draftTimer = setTimeout(() => {
			try {
				localStorage.setItem(DRAFT_KEY, snapshot);
			} catch {
				// ignore quota errors
			}
		}, 300);
	});

	function reset() {
		title = '';
		description = '';
		status = defaultSettings.defaultStatus;
		priority = String(defaultSettings.defaultPriority);
		projectId = projects[0]?.id ?? '';
		dueDate = '';
		endDate = '';
		recurrenceRule = '';
		estimateText = '';
		error = null;
		createMore = false;
		attachments = [];
		selectedLabelIds = [];
		previewMode = false;
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
				attachments = [
					...attachments,
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

	function removeAttachment(index: number) {
		attachments = attachments.filter((_, i) => i !== index);
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const trimmedTitle = title.trim();

		if (!trimmedTitle) {
			error = 'Title is required';
			return;
		}
		const project = projects.find((item) => item.id === projectId);
		if (!project) {
			error = 'Project is required';
			return;
		}

		if (estimateText.trim() && parseEstimate(estimateText) === null) {
			error = 'Invalid estimate (try 90m, 2h or 1h30m)';
			return;
		}

		error = null;
		submitting = true;

		try {
			const task = await create({
				title: trimmedTitle,
				projectId: project.id,
				description: description.trim() || null,
				status,
				priority: Number(priority) as TaskPriority,
				dueDate: dueDate || null,
				endDate: endDate || null,
				reminderAt: reminderAt || null,
				recurrenceRule: recurrenceRule || null,
				parentId: initialParentId ?? null,
				estimateMinutes: parseEstimate(estimateText)
			});

			clearDraft();

			for (const att of attachments) {
				await createAttachment({
					taskId: task.id,
					fileName: att.fileName,
					fileData: att.fileData,
					mimeType: att.mimeType,
					fileSize: att.fileSize
				});
			}

			if (selectedLabelIds.length > 0) {
				await setTaskLabels(task.id, selectedLabelIds);
			}

			onCreated?.(task);
			if (createMore) {
				title = '';
				description = '';
				error = null;
				attachments = [];
				selectedLabelIds = [];
				dueDate = '';
				endDate = '';
				recurrenceRule = '';
				estimateText = '';
				previewMode = false;
				requestAnimationFrame(() => titleRef?.focus());
			} else {
				open = false;
				reset();
			}
		} catch {
			error = 'Failed to create task';
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content
		class="w-[calc(100vw-2rem)] max-w-lg gap-0 p-0"
		showCloseButton={false}
		onInteractOutside={(e) => {
			// keep the dialog open while the lightbox is showing
			if (lightboxUrl) e.preventDefault();
		}}
	>
		<Dialog.Title class="sr-only">Create task</Dialog.Title>
		<form onsubmit={handleSubmit} class="flex flex-col">
			<!-- header: badge + breadcrumb + close -->
			<header
				class="flex items-center justify-between gap-2 border-b border-border/60 px-4 py-3 sm:px-5"
			>
				<div class="flex min-w-0 items-center gap-2">
					<div
						class="flex size-4.5 shrink-0 items-center justify-center rounded-[5px] bg-primary text-[9px] leading-none font-bold text-primary-foreground"
					>
						T
					</div>
					<span class="truncate text-[12px] text-muted-foreground">
						New task · {projects.find((p) => p.id === projectId)?.name ?? 'Inbox'}
					</span>
				</div>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:text-foreground"
						>
							<Icon icon="close" size={16} />
						</Button>
					{/snippet}
				</Dialog.Close>
			</header>

			<!-- body -->
			<div class="flex flex-col gap-4 px-4 py-4 sm:px-5">
				<Input
					bind:ref={titleRef}
					bind:value={title}
					placeholder="Task title"
					class="h-auto px-3 py-2 text-[18px] font-semibold placeholder:text-muted-foreground/40"
				/>

				<!-- description -->
				<section>
					<div class="flex items-center justify-between pb-1">
						<span class="text-[11px] font-medium text-muted-foreground/60">Description</span>
						{#if description.trim()}
							<button
								type="button"
								class="flex items-center gap-1 text-[11px] text-muted-foreground/60 transition-colors hover:text-foreground"
								onclick={() => (previewMode = !previewMode)}
							>
								{#if previewMode}
									<Icon icon="pencil" size={10} />
									<span>Edit</span>
								{:else}
									<Icon icon="eye" size={10} />
									<span>Preview</span>
								{/if}
							</button>
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
							rows={2}
							class="min-h-0 text-[13px] placeholder:text-muted-foreground/50"
						/>
					{/if}
				</section>

				{#if attachments.length > 0}
					<div class="flex flex-wrap gap-2">
						{#each attachments as att, i (i)}
							<div
								class="group/att relative size-16 cursor-zoom-in overflow-hidden rounded-lg border border-border bg-muted/30"
								role="button"
								tabindex="0"
								onclick={() => {
									if (att.fileData) lightboxUrl = att.fileData;
									lightboxAtt = att;
								}}
								onkeydown={(e) => {
									if (e.key === 'Enter' && att.fileData) lightboxUrl = att.fileData;
									lightboxAtt = att;
								}}
							>
								<img src={att.fileData} alt={att.fileName} class="size-full object-cover" />
								<Button
									variant="ghost"
									size="icon-xs"
									onclick={() => removeAttachment(i)}
									class="absolute top-1 right-1 bg-background/80 text-foreground opacity-0 transition-opacity group-hover/att:opacity-100"
									aria-label="Remove attachment"
								>
									<Icon icon="close" size={10} />
								</Button>
							</div>
						{/each}
					</div>
				{/if}

				<!-- metadata: pills on top, pickers in pairs -->
				<div class="flex flex-col gap-1.5">
					<div class="flex flex-wrap gap-1.5">
						<Select.Root type="single" bind:value={projectId as never}>
							<Select.Trigger
								size="sm"
								class="gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none hover:bg-muted/50"
							>
								<span class="size-2 rounded-full bg-primary/80"></span>
								{projects.find((p) => p.id === projectId)?.name ?? 'Select project'}
							</Select.Trigger>
							<Select.Content>
								{#each projects as project (project.id)}
									<Select.Item value={project.id} label={`${project.name} (${project.prefix})`} />
								{/each}
							</Select.Content>
						</Select.Root>

						<Select.Root type="single" bind:value={status as never}>
							<Select.Trigger
								size="sm"
								class="gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none hover:bg-muted/50"
							>
								<StatusIcon {status} size={12} />
								{statusLabels[status]}
							</Select.Trigger>
							<Select.Content>
								{#each Object.entries(statusLabels) as [value, label] (value)}
									<Select.Item {value} {label}>
										<StatusIcon status={value as TaskStatus} size={12} />
										<span>{label}</span>
									</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>

						<Select.Root type="single" bind:value={priority as never}>
							<Select.Trigger
								size="sm"
								class="gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none hover:bg-muted/50"
							>
								{#if priority !== '0'}
									<PriorityIcon priority={Number(priority)} size={13} />
								{/if}
								{priorityLabels[priority]}
							</Select.Trigger>
							<Select.Content>
								{#each Object.entries(priorityLabels) as [value, label] (value)}
									<Select.Item {value} {label}>
										<PriorityIcon priority={Number(value)} size={14} />
										<span>{label}</span>
									</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>
					</div>

					<LabelSelector
						bind:selectedIds={selectedLabelIds}
						{labels}
						onCreated={onLabelCreated}
						onUpdated={onLabelUpdated}
						onRemoved={onLabelRemoved}
					/>

					<div class="grid grid-cols-2 gap-1.5">
						<DueDatePicker
							value={dueDate}
							onSelect={(d) => (dueDate = d)}
							onClear={() => (dueDate = '')}
						/>
						<DueDatePicker
							title="End date"
							value={endDate}
							onSelect={(d) => (endDate = d)}
							onClear={() => (endDate = '')}
						/>
					</div>

					<div class="grid grid-cols-2 gap-1.5">
						<RecurrencePicker
							value={recurrenceRule}
							anchorDate={dueDate || null}
							onSelect={(rule) => (recurrenceRule = rule)}
							onClear={() => (recurrenceRule = '')}
						/>
						<ReminderPicker
							value={reminderAt}
							onSelect={(iso) => (reminderAt = iso)}
							onClear={() => (reminderAt = '')}
						/>
					</div>

					<div class="relative">
						<Icon
							icon="stopwatch"
							size={13}
							class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground/50"
						/>
						<Input
							bind:value={estimateText}
							placeholder="Estimate (90m, 2h, 1h30m)"
							class="h-8 w-full rounded-lg border-input bg-transparent pr-3 pl-8 text-[12px] text-foreground outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
						/>
					</div>
				</div>

				{#if error}
					<p class="text-[12px] text-destructive" role="alert">{error}</p>
				{/if}
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
			<footer
				class="flex flex-wrap items-center justify-between gap-2 border-t border-border/60 px-4 py-3 sm:px-5"
			>
				<div class="flex items-center gap-2">
					<Checkbox id="create-more" bind:checked={createMore} />
					<FormLabel
						for="create-more"
						class="cursor-pointer text-[12px] font-normal text-muted-foreground"
						>Create more</FormLabel
					>
				</div>
				<div class="flex items-center gap-2">
					<Button
						type="button"
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground"
						tabindex={-1}
						aria-label="Attach"
						onclick={() => fileInput?.click()}
					>
						{#if uploading}
							<Spinner class="size-4" />
						{:else}
							<Icon icon="paperclip" size={16} />
						{/if}
					</Button>
					<Button type="submit" size="sm" disabled={submitting || !title.trim()}>
						{submitting ? 'Creating...' : 'Create task'}
					</Button>
				</div>
			</footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<Lightbox
	url={lightboxUrl}
	onClose={() => {
		lightboxUrl = null;
		lightboxAtt = null;
	}}
	onDownload={() => {
		if (lightboxAtt) downloadDataUrl(lightboxAtt.fileData, lightboxAtt.fileName);
	}}
/>
