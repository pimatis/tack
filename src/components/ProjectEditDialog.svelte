<script lang="ts">
	import { update } from '$lib/repositories/project.repository';
	import type { Project } from '$lib/types/project';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import MarkdownRenderer from './MarkdownRenderer.svelte';
	import { normalizePrefix, isValidPrefix } from '$lib/prefix';

	type Props = { open?: boolean; project: Project | null; onUpdated?: (project: Project) => void };
	let { open = $bindable(false), project, onUpdated }: Props = $props();
	let name = $state('');
	let prefix = $state('');
	let description = $state('');
	let previewMode = $state(false);
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let nameRef = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (!open || !project) return;
		name = project.name;
		prefix = project.prefix;
		description = project.description ?? '';
		previewMode = false;
		error = null;
	});

	$effect(() => {
		if (open) requestAnimationFrame(() => nameRef?.focus());
	});

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!project || !name.trim()) {
			error = 'Project name is required';
			return;
		}
		const normalizedPrefix = normalizePrefix(prefix);
		if (!isValidPrefix(normalizedPrefix)) {
			error = 'Prefix must be 2-4 letters or numbers';
			return;
		}
		submitting = true;
		try {
			const updated = await update(project.id, {
				name: name.trim(),
				prefix: normalizedPrefix,
				description: description.trim() || null
			});
			if (updated) onUpdated?.(updated);
			open = false;
		} catch {
			error = 'Failed to update project. Prefix may already be in use';
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-md gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Edit project</Dialog.Title>
		<form onsubmit={handleSubmit} class="flex flex-col">
			<!-- header -->
			<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
				<span class="text-[13px] font-medium text-foreground">Edit project</span>
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
				<Button type="submit" size="sm" disabled={submitting}>
					{submitting ? 'Saving...' : 'Save changes'}
				</Button>
			</div>
		</form>
	</Dialog.Content>
</Dialog.Root>
