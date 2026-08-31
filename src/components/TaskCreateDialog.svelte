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
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import LabelSelector from './LabelSelector.svelte';
	import MarkdownRenderer from './MarkdownRenderer.svelte';
	import DueDatePicker from './DueDatePicker.svelte';
	import { getSettings } from '$lib/stores/settings';

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

	$effect(() => {
		if (open && !projectId && projects.length > 0) projectId = projects[0].id;
	});

	$effect(() => {
		if (open) dueDate = initialDueDate ?? '';
	});

	$effect(() => {
		if (open) requestAnimationFrame(() => titleRef?.focus());
	});

	function reset() {
		title = '';
		description = '';
		status = defaultSettings.defaultStatus;
		priority = String(defaultSettings.defaultPriority);
		projectId = projects[0]?.id ?? '';
		dueDate = '';
		endDate = '';
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
				endDate: endDate || null
			});

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
		class="max-w-lg gap-0 p-0"
		showCloseButton={false}
		onInteractOutside={(e) => {
			// keep the dialog open while the lightbox is showing
			if (lightboxUrl) e.preventDefault();
		}}
	>
		<Dialog.Title class="sr-only">Create task</Dialog.Title>
		<form onsubmit={handleSubmit} class="flex flex-col">
			<!-- header: badge + breadcrumb + close -->
			<div class="flex items-center justify-between px-5 pt-4 pb-3">
				<div class="flex items-center gap-2">
					<div
						class="flex size-4.5 items-center justify-center rounded-[5px] bg-primary text-[9px] leading-none font-bold text-primary-foreground"
					>
						T
					</div>
					<span class="text-[12px] text-muted-foreground">
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
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
								/></svg
							>
						</Button>
					{/snippet}
				</Dialog.Close>
			</div>

			<!-- title + description -->
			<div class="flex flex-col gap-2 px-5">
				<Input
					bind:ref={titleRef}
					bind:value={title}
					placeholder="Task title"
					class="h-auto border-none bg-transparent px-3 py-1 text-[18px] font-semibold shadow-none placeholder:text-muted-foreground/40"
				/>
				<div class="flex flex-col gap-1.5">
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
							rows={2}
							class="min-h-0 border-none bg-transparent px-3 py-1 text-[13px] shadow-none placeholder:text-muted-foreground/50"
						/>
					{/if}
				</div>
			</div>

			{#if attachments.length > 0}
				<div class="flex flex-wrap gap-2 px-5 pt-1">
					{#each attachments as att, i (i)}
						<div
							class="group/att relative size-16 cursor-zoom-in overflow-hidden rounded-lg border border-border bg-muted/30"
							role="button"
							tabindex="0"
							onclick={() => {
								if (att.fileData) lightboxUrl = att.fileData;
							}}
							onkeydown={(e) => {
								if (e.key === 'Enter' && att.fileData) lightboxUrl = att.fileData;
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
			{/if}

			<Input
				bind:ref={fileInput}
				type="file"
				accept="image/*"
				multiple
				class="hidden"
				onchange={handleFileSelect}
			/>

			<!-- metadata pills -->
			<div class="flex flex-wrap gap-1.5 px-5 pt-3 pb-4">
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

				<LabelSelector
					bind:selectedIds={selectedLabelIds}
					{labels}
					onCreated={onLabelCreated}
					onUpdated={onLabelUpdated}
					onRemoved={onLabelRemoved}
				/>

				<!-- due date picker -->
				<DueDatePicker
					value={dueDate}
					onSelect={(d) => (dueDate = d)}
					onClear={() => (dueDate = '')}
				/>

				<!-- end date picker -->
				<DueDatePicker
					title="End date"
					value={endDate}
					onSelect={(d) => (endDate = d)}
					onClear={() => (endDate = '')}
				/>
			</div>

			{#if error}
				<p class="px-5 text-[12px] text-destructive" role="alert">{error}</p>
			{/if}

			<Separator />

			<!-- footer -->
			<div class="flex items-center justify-between px-5 py-3">
				<div class="flex items-center gap-2">
					<Checkbox id="create-more" bind:checked={createMore} />
					<label for="create-more" class="cursor-pointer text-[12px] text-muted-foreground"
						>Create more</label
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
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M15.889 9.525a1.5 1.5 0 0 1 2.007-.103l.114.103 2.122 2.121a6 6 0 0 1-8.303 8.661l-.183-.175-2.121-2.122a1.5 1.5 0 0 1 2.007-2.224l.114.103 2.122 2.121a3 3 0 0 0 4.377-4.098l-.135-.144-2.121-2.122a1.5 1.5 0 0 1 0-2.121m-7.071-.707a1.5 1.5 0 0 1 2.007-.103l.114.103 4.243 4.243a1.5 1.5 0 0 1-2.008 2.224l-.114-.103-4.242-4.243a1.5 1.5 0 0 1 0-2.121m-4.95-4.95a6 6 0 0 1 8.302-.175l.184.175 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.114-.103-2.12-2.121a3 3 0 0 0-4.378 4.098l.135.144 2.12 2.122a1.5 1.5 0 0 1-2.007 2.224l-.113-.103-2.122-2.121a6 6 0 0 1 0-8.486"
								/></svg
							>
						{/if}
					</Button>
					<Button type="submit" size="sm" disabled={submitting}>
						{submitting ? 'Creating...' : 'Create task'}
					</Button>
				</div>
			</div>
		</form>
	</Dialog.Content>
</Dialog.Root>

<Lightbox url={lightboxUrl} onClose={() => (lightboxUrl = null)} />
