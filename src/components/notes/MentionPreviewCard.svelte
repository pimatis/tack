<script lang="ts">
	import { Card } from '$lib/components/ui/card/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import PriorityIcon from '../PriorityIcon.svelte';
	import MarkdownRenderer from '../MarkdownRenderer.svelte';
	import { TaskPageState } from '$lib/task/taskState.svelte';
	import { statusConfig, priorityConfig } from '$lib/task/constants';
	import { notesInvoke } from '$lib/notes/liveNotes';

	// mention href: "task:<id>" or "note:<encoded path>"
	let { href }: { href: string } = $props();

	const kind = $derived(href.startsWith('note:') ? 'note' : 'task');
	// note hrefs may carry a #L<line> fragment; the preview reads the whole note
	const notePath = $derived(
		kind === 'note' ? decodeURIComponent(href.slice(5)).replace(/#L\d+$/, '') : ''
	);
	const task = $derived.by(() => {
		if (kind !== 'task') return undefined;
		const id = decodeURIComponent(href.slice(5));
		// tasks may not be loaded yet on first hover
		void TaskPageState.get().refresh();
		return TaskPageState.get().tasks.find((t) => t.id === id);
	});

	// note content is read on open; a local fs read per hover is cheap enough
	let noteContent = $state<string | null>(null);
	let noteError = $state(false);
	$effect(() => {
		if (kind !== 'note') return;
		const path = notePath;
		noteContent = null;
		noteError = false;
		void notesInvoke<string>('read_file', { path }).then(
			(content) => {
				// ignore stale reads if the hover target changed meanwhile
				if (notePath === path) noteContent = content;
			},
			() => {
				if (notePath === path) noteError = true;
			}
		);
	});

	const noteTitle = $derived(notePath.split('/').pop()?.replace(/\.md$/, '') ?? '');
</script>

<Card class="flex w-80 flex-col gap-2 p-3 shadow-xl">
	{#if kind === 'task'}
		{#if task}
			<div class="flex items-center gap-2">
				<StatusIcon status={task.status} size={14} />
				<PriorityIcon priority={task.priority} size={14} />
				<span class="truncate text-[13px] font-medium text-foreground">{task.title}</span>
			</div>
			<div class="flex items-center gap-2 text-[11px] text-muted-foreground">
				<span>{statusConfig[task.status].label}</span>
				<span>·</span>
				<span>{priorityConfig[task.priority].label}</span>
			</div>
			{#if task.description}
				<p class="line-clamp-6 text-[12px] leading-relaxed text-foreground/80">
					{task.description}
				</p>
			{/if}
		{:else}
			<p class="text-[12px] text-muted-foreground">Loading task…</p>
		{/if}
	{:else}
		<p class="truncate text-[13px] font-medium text-foreground">{noteTitle}</p>
		<div class="relative max-h-72 overflow-hidden">
			{#if noteContent === null}
				<p class="text-[12px] text-muted-foreground">
					{noteError ? 'Cannot load note.' : 'Loading…'}
				</p>
			{:else}
				<!-- static peek: clicks inside would be misleading, the card opens the note -->
				<div class="pointer-events-none prose prose-sm max-w-none opacity-90 prose-invert">
					<MarkdownRenderer content={noteContent} />
				</div>
				<div
					class="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-card to-transparent"
				></div>
			{/if}
		</div>
	{/if}
</Card>
