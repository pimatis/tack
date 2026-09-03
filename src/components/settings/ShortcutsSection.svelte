<!-- editable keyboard shortcuts: click a row to record a new key combo, save applies all changes -->
<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { SHORTCUTS, shortcutIdLabel, comboFromEvent, combosEqual } from '$lib/shortcuts/index.js';
	import type { ShortcutKey } from '$lib/shortcuts/index.js';
	import { getSettings, setSettings } from '$lib/stores/settings';
	import { defaultSettings } from '$lib/types/settings';
	import ShortcutKeycap from '../ShortcutKeycap.svelte';

	let draft = $state<Record<string, ShortcutKey[]>>(
		JSON.parse(JSON.stringify(getSettings().shortcuts))
	);
	let dialogOpen = $state(false);
	let recordingId = $state<string | null>(null);
	let pendingCombo = $state<ShortcutKey | null>(null);
	let conflictId = $state<string | null>(null);
	let savedFlash = $state(false);

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(getSettings().shortcuts));
	// reset stays available while saved values differ from defaults (not only while draft is dirty)
	const canReset = $derived(
		dirty || JSON.stringify(getSettings().shortcuts) !== JSON.stringify(defaultSettings.shortcuts)
	);
	const recording = $derived(SHORTCUTS.find((s) => s.id === recordingId) ?? null);

	// keep draft in sync when settings change elsewhere (cli sync) while nothing is pending
	$effect(() => {
		const saved = getSettings().shortcuts;
		if (!dirty && JSON.stringify(draft) !== JSON.stringify(saved)) {
			draft = JSON.parse(JSON.stringify(saved));
		}
	});

	function openRecorder(id: string) {
		recordingId = id;
		pendingCombo = null;
		conflictId = null;
		dialogOpen = true;
	}

	function closeRecorder() {
		dialogOpen = false;
		recordingId = null;
		pendingCombo = null;
		conflictId = null;
	}

	function findConflict(combo: ShortcutKey, exceptId: string): string | null {
		for (const [id, keys] of Object.entries(draft)) {
			if (id === exceptId) continue;
			if (keys.some((k) => combosEqual(k, combo))) return id;
		}
		return null;
	}

	function applyCombo(combo: ShortcutKey) {
		if (conflictId) draft[conflictId] = [];
		if (recordingId) draft[recordingId] = [combo];
		closeRecorder();
	}

	function handleRecord(e: KeyboardEvent) {
		// ignore keys while another app has focus
		if (!document.hasFocus()) return;
		e.preventDefault();
		e.stopPropagation();
		if (e.key === 'Escape') {
			closeRecorder();
			return;
		}
		if (e.key === 'Backspace' || e.key === 'Delete') {
			if (recordingId) draft[recordingId] = [];
			closeRecorder();
			return;
		}
		const combo = comboFromEvent(e);
		if (!combo) return;
		const conflict = recordingId ? findConflict(combo, recordingId) : null;
		if (conflict) {
			pendingCombo = combo;
			conflictId = conflict;
			return;
		}
		applyCombo(combo);
	}

	$effect(() => {
		if (!dialogOpen) return;
		const onKeydown = (e: KeyboardEvent) => handleRecord(e);
		window.addEventListener('keydown', onKeydown, true);
		return () => window.removeEventListener('keydown', onKeydown, true);
	});

	function save() {
		setSettings({ shortcuts: JSON.parse(JSON.stringify(draft)) });
		savedFlash = true;
		setTimeout(() => (savedFlash = false), 1600);
	}

	function reset() {
		draft = JSON.parse(JSON.stringify(defaultSettings.shortcuts));
	}
</script>

<Card.Root size="sm" class="!py-0">
	<!-- header -->
	<div class="flex flex-wrap items-start justify-between gap-3 px-4 pt-3.5 pb-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Keyboard shortcuts</p>
			<p class="mt-0.5 text-xs text-muted-foreground">
				Click any shortcut to record a new key combination
			</p>
		</div>
		<Button size="sm" class="shrink-0" disabled={!dirty} onclick={save}>
			{savedFlash ? 'Saved' : 'Save changes'}
		</Button>
	</div>
	<Separator />

	<!-- shortcut rows -->
	{#each SHORTCUTS as s, i (s.id)}
		<Button
			variant="ghost"
			class="h-auto w-full justify-between gap-3 rounded-none px-4 py-2.5 font-normal text-foreground/90 hover:bg-muted/40"
			onclick={() => openRecorder(s.id)}
		>
			<span class="min-w-0 flex-1 truncate text-left text-[13px] text-foreground/90">{s.label}</span
			>
			<span class="flex shrink-0 items-center gap-2.5">
				{#if draft[s.id]?.length}
					<ShortcutKeycap combo={draft[s.id][0]} />
				{:else}
					<span class="text-[11px] text-muted-foreground/50">No shortcut</span>
				{/if}
				<svg
					class="text-muted-foreground/40"
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
				>
					<rect
						x="2"
						y="6"
						width="20"
						height="12"
						rx="2"
						stroke="currentColor"
						stroke-width="1.5"
					/>
					<path
						d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M18 14h.01M9 14h6"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
					/>
				</svg>
			</span>
		</Button>
		{#if i < SHORTCUTS.length - 1}<Separator />{/if}
	{/each}

	<!-- footer -->
	<div class="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5">
		<span class="text-[11px] text-muted-foreground/50">
			{Object.keys(draft).length} shortcuts
		</span>
		<Button
			variant="ghost"
			size="sm"
			class="text-[12px] font-normal text-muted-foreground"
			disabled={!canReset}
			onclick={reset}
		>
			Reset to defaults
		</Button>
	</div>
</Card.Root>

<!-- record dialog -->
<Dialog.Root
	bind:open={dialogOpen}
	onOpenChange={(o) => {
		dialogOpen = o;
		if (!o) closeRecorder();
	}}
>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="px-5 pt-4 text-[14px] font-semibold">Keyboard shortcut</Dialog.Title>
		<Dialog.Description class="px-5 pt-1 text-xs text-muted-foreground">
			Press keys to set a shortcut for {recording?.label}
		</Dialog.Description>
		<div class="px-5 py-4">
			<div
				class="flex h-14 items-center justify-center rounded-lg border border-border bg-muted/30 transition-colors"
			>
				{#if pendingCombo}
					<ShortcutKeycap combo={pendingCombo} size="lg" />
				{:else}
					<span class="text-[12px] text-muted-foreground/60">press keys to record</span>
				{/if}
			</div>
			{#if conflictId}
				<div
					class="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2"
				>
					<span class="text-xs text-amber-500">
						{shortcutIdLabel(conflictId)} already uses this shortcut
					</span>
					<Button
						size="sm"
						variant="ghost"
						class="shrink-0 text-[12px] font-normal text-amber-500 hover:bg-amber-500/10 hover:text-amber-400"
						onclick={() => pendingCombo && applyCombo(pendingCombo)}
					>
						Use anyway
					</Button>
				</div>
			{/if}
		</div>
		<Dialog.Footer class="flex items-center justify-between border-t border-border/60 px-5 py-3">
			<span class="text-[11px] text-muted-foreground/50"> esc to cancel · backspace to clear </span>
			<Button variant="ghost" size="sm" class="text-[12px] font-normal" onclick={closeRecorder}>
				Cancel
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
