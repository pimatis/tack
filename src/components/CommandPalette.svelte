<script lang="ts">
	import { onMount } from 'svelte';
	import { findAll as findAllTasks, findTrashed } from '$lib/repositories/task.repository';
	import { findAll as findAllProjects } from '$lib/repositories/project.repository';
	import { findAll as findAllLabels } from '$lib/repositories/label.repository';
	import * as Command from '$lib/components/ui/command/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import { paletteProviders, type PaletteContext, type PaletteItem } from '$lib/palette/index.js';
	import { goHomeThenDispatch } from '$lib/palette/utils';
	import { pushRecent } from '$lib/palette/recents';
	import { getSettings, setSettings } from '$lib/stores/settings';
	import type { Theme } from '$lib/types/settings';
	import type { ColorPresetId } from '$lib/theme/presets';
	import StatusIcon from './task/StatusIcon.svelte';
	import Icon from '$lib/components/Icon.svelte';

	// ">" actions, "#" projects, "@" labels, "?" filters; everything else
	// searches all providers at once
	const PREFIX_SCOPES: Record<string, string[]> = {
		'>': ['actions'],
		'#': ['projects'],
		'@': ['labels'],
		'?': ['filters']
	};

	let open = $state(false);
	let query = $state('');
	let ctx = $state<PaletteContext>({ tasks: [], projects: [], labels: [], trash: [] });
	let results = $state<Record<string, PaletteItem[]>>({});
	let highlighted = $state('');

	const parsed = $derived.by(() => {
		const trimmed = query.trim();
		const ids = PREFIX_SCOPES[trimmed[0] ?? ''];
		if (ids) return { ids, q: trimmed.slice(1).trim(), scoped: true };
		return { ids: null as string[] | null, q: trimmed, scoped: false };
	});

	const activeProviders = $derived(
		parsed.ids ? paletteProviders.filter((p) => parsed.ids!.includes(p.id)) : paletteProviders
	);

	const visibleGroups = $derived(
		activeProviders
			.map((provider) => ({ provider, items: results[provider.id] ?? [] }))
			.filter((group) => group.items.length > 0)
	);

	const resolveItem = (providerId: string, itemId: string) =>
		paletteProviders.find((p) => p.id === providerId)?.resolve?.(itemId, ctx) ?? null;

	async function loadData() {
		try {
			const [tasks, projects, labels, trash] = await Promise.all([
				findAllTasks(),
				findAllProjects(),
				findAllLabels(),
				findTrashed()
			]);
			ctx = {
				tasks,
				projects,
				labels,
				trash,
				refresh: () => void loadData(),
				resolve: resolveItem
			};
		} catch {
			// ignore
		}
	}

	// theme preview state: snapshot the real settings on open, restore them on
	// close unless a themes item was committed with enter/click
	let previewBase: { theme: Theme; themePreset: ColorPresetId } | null = null;
	let previewApplied = false;

	$effect(() => {
		if (!open) return;
		previewBase = { theme: getSettings().theme, themePreset: getSettings().themePreset };
		previewApplied = false;
		void loadData();
	});

	$effect(() => {
		if (open) return;
		if (previewApplied && previewBase) setSettings(previewBase);
		previewApplied = false;
		previewBase = null;
	});

	// debounced provider search; the seq guard drops stale async results
	let searchSeq = 0;
	$effect(() => {
		const { q, scoped } = parsed;
		const c = ctx;
		if (!open) {
			results = {};
			return;
		}
		const seq = ++searchSeq;
		const timer = setTimeout(
			async () => {
				const entries = await Promise.all(
					activeProviders.map(async (provider) => {
						const items = await provider.search(q, c, scoped);
						return [provider.id, items] as const;
					})
				);
				if (seq === searchSeq) results = Object.fromEntries(entries);
			},
			q ? 150 : 0
		);
		return () => clearTimeout(timer);
	});

	function highlightedItem(): PaletteItem | null {
		const sep = highlighted.indexOf(':');
		if (sep === -1) return null;
		const group = results[highlighted.slice(0, sep)];
		return group?.find((item) => item.id === highlighted.slice(sep + 1)) ?? null;
	}

	// live theme preview while the highlight moves over themes items
	$effect(() => {
		if (!open) return;
		const item = highlightedItem();
		if (!item?.preview) return;
		item.preview();
		previewApplied = true;
	});

	function select(providerId: string, item: PaletteItem) {
		// a selected preview item commits the preview; anything else lets the
		// close-watcher restore the snapshot
		if (item.preview) previewApplied = false;
		if (!item.noRecent) pushRecent(item.providerId ?? providerId, item.id);
		void item.run();
		open = false;
	}

	// mod+enter / alt+enter run an item's sub-action instead of selecting it
	function handleKeydown(e: KeyboardEvent) {
		if (e.key !== 'Enter') return;
		const combo = e.altKey ? 'alt-enter' : e.metaKey || e.ctrlKey ? 'mod-enter' : null;
		if (!combo) return;
		const action = highlightedItem()?.actions?.find((a) => a.combo === combo);
		if (!action) return;
		e.preventDefault();
		e.stopPropagation();
		void action.run();
	}

	function newTask() {
		void goHomeThenDispatch('open-task-dialog');
		open = false;
	}

	function newProject() {
		void goHomeThenDispatch('open-project-dialog');
		open = false;
	}

	onMount(() => {
		const registry = getShortcutRegistry();

		const togglePalette = () => (open = !open);

		const unregisterCommandPalette = registry.register({
			id: 'command-palette',
			run: togglePalette
		});

		const unregisterQuickOpen = registry.register({
			id: 'quick-open',
			run: togglePalette
		});

		const unregisterNewTask = registry.register({
			id: 'new-task',
			enabled: () =>
				window.location.pathname === '/' &&
				(open || !document.querySelector("[role='dialog'] input")),
			run: () => newTask()
		});

		const unregisterNewProject = registry.register({
			id: 'new-project',
			enabled: () =>
				window.location.pathname === '/' &&
				(open || !document.querySelector("[role='dialog'] input")),
			run: () => newProject()
		});

		window.addEventListener('open-command-palette', togglePalette);
		return () => {
			unregisterCommandPalette();
			unregisterQuickOpen();
			unregisterNewTask();
			unregisterNewProject();
			window.removeEventListener('open-command-palette', togglePalette);
		};
	});
</script>

<Command.Dialog
	bind:open
	bind:value={highlighted}
	shouldFilter={false}
	title="Search"
	description="Search tasks, projects, filters, themes and settings"
	showCloseButton={false}
	class="top-[12%]! w-[calc(100vw-2rem)]! max-w-[560px]! sm:top-[18%]"
>
	<Command.Input
		bind:value={query}
		placeholder="Search everything... (> actions · # projects · @ labels · ? filters)"
		onkeydown={handleKeydown}
	/>
	<Command.List class="max-h-[60vh] sm:max-h-[400px]">
		<Command.Empty>No results found.</Command.Empty>
		{#each visibleGroups as group, i (group.provider.id)}
			{#if i > 0}<Command.Separator />{/if}
			<Command.Group value={group.provider.id}>
				<Collapsible.Root open class="group/palette">
					<Collapsible.Trigger
						class="flex w-full cursor-pointer list-none items-center gap-1 rounded-sm px-2 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
					>
						<Icon
							icon="arrow-right"
							size={14}
							class="text-muted-foreground transition-transform duration-150 group-data-[state=open]/palette:rotate-90"
						/>
						<span>{group.provider.heading}</span>
						<span class="ml-auto text-[11px] text-muted-foreground/50">{group.items.length}</span>
					</Collapsible.Trigger>
					<Collapsible.Content>
						{#each group.items as item (item.id)}
							<Command.Item
								value="{group.provider.id}:{item.id}"
								onSelect={() => select(group.provider.id, item)}
								class="[&_.cn-command-item-indicator]:hidden"
							>
								{#if item.dot}
									<span class="size-2 shrink-0 rounded-full {item.dot}"></span>
								{:else if item.status}
									<StatusIcon status={item.status} size={12} />
								{:else if item.icon}
									<Icon icon={item.icon} size={14} class="text-muted-foreground" />
								{/if}
								<span class="min-w-0 flex-1 truncate">{item.label}</span>
								{#if item.actions}
									{#each item.actions as action (action.combo)}
										<span
											class="shrink-0 rounded border border-border/50 px-0.5 text-[9px] leading-4 text-muted-foreground/50"
											title={action.label}>{action.hint}</span
										>
									{/each}
								{/if}
								{#if item.hint}
									<span class="ml-auto shrink-0 font-mono text-[11px] text-muted-foreground/50"
										>{item.hint}</span
									>
								{/if}
							</Command.Item>
						{/each}
					</Collapsible.Content>
				</Collapsible.Root>
			</Command.Group>
		{/each}
	</Command.List>
</Command.Dialog>
