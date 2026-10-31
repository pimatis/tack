<script lang="ts">
	import { create } from '$lib/repositories/project.repository';
	import type { Project } from '$lib/types/project';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import MarkdownRenderer from './MarkdownRenderer.svelte';
	import { generatePrefix, normalizePrefix, isValidPrefix } from '$lib/prefix';
	import Icon from '$lib/components/Icon.svelte';

	type Props = {
		open?: boolean;
		onCreated?: (project: Project) => void;
	};

	let { open = $bindable(false), onCreated }: Props = $props();
	let name = $state('');
	let prefix = $state('');
	let description = $state('');
	let previewMode = $state(false);
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let nameRef = $state<HTMLInputElement | null>(null);

	// auto-generate prefix from name
	$effect(() => {
		if (open) requestAnimationFrame(() => nameRef?.focus());
	});

	$effect(() => {
		if (open) prefix = generatePrefix(name);
	});

	function reset() {
		name = '';
		prefix = '';
		description = '';
		previewMode = false;
		error = null;
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const trimmedName = name.trim();
		const normalizedPrefix = normalizePrefix(prefix);

		if (!trimmedName) {
			error = 'Project name is required';
			return;
		}
		if (!isValidPrefix(normalizedPrefix)) {
			error = 'Prefix must be 2-4 letters or numbers';
			return;
		}

		submitting = true;
		error = null;
		try {
			const project = await create({
				name: trimmedName,
				prefix: normalizedPrefix,
				description: description.trim() || null
			});
			onCreated?.(project);
			open = false;
			reset();
		} catch {
			error = 'Failed to create project. Prefix may already be in use';
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-md gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Create project</Dialog.Title>
		<form onsubmit={handleSubmit} class="flex flex-col">
			<!-- header -->
			<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
				<span class="text-[13px] font-medium text-foreground">New project</span>
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
			</div>

			<!-- body -->
			<div class="flex flex-col gap-3 px-4 pb-4 sm:px-5">
				<Input
					bind:ref={nameRef}
					bind:value={name}
					placeholder="Project name"
					class="h-auto border-none bg-transparent px-3 py-1 text-[18px] font-semibold shadow-none placeholder:text-muted-foreground/40"
				/>
				<div class="flex items-center gap-2">
					<span class="shrink-0 text-[12px] text-muted-foreground">Prefix</span>
					<Input
						bind:value={prefix}
						placeholder="WEB"
						maxlength={4}
						disabled
						class="h-7 w-20 text-[12px] font-medium"
					/>
				</div>

				<!-- description with markdown support -->
				<div class="flex flex-col gap-1.5">
					<div class="flex items-center justify-between">
						<span class="text-[12px] text-muted-foreground">Description</span>
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
						{/if}
					</div>
					{#if previewMode}
						<div class="min-h-[80px] rounded-lg border border-border bg-muted/20 p-3">
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
							rows={3}
							class="min-h-[80px] border-none bg-transparent px-3 py-1 text-[13px] shadow-none placeholder:text-muted-foreground/50"
						/>
					{/if}
				</div>

				<p class="text-[11px] text-muted-foreground/60">
					Tasks in this project will use this prefix as an identifier.
				</p>
			</div>

			{#if error}
				<p class="px-4 text-[12px] text-destructive sm:px-5" role="alert">{error}</p>
			{/if}

			<Separator />

			<!-- footer -->
			<div class="flex flex-wrap items-center justify-end gap-2 px-4 py-3 sm:px-5">
				<Button type="button" variant="ghost" size="sm" onclick={() => (open = false)}
					>Cancel</Button
				>
				<Button type="submit" size="sm" disabled={submitting || !name.trim()}>
					{submitting ? 'Creating...' : 'Create project'}
				</Button>
			</div>
		</form>
	</Dialog.Content>
</Dialog.Root>
