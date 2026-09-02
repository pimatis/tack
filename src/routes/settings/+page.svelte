<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { getSettings, setSettings, applyTheme } from '$lib/stores/settings';
	import { onDbChanged } from '$lib/db/client';
	import type { Settings, Theme } from '$lib/types/settings';
	import { findAll as findAllProjects } from '$lib/repositories/project.repository';
	import { findAll as findAllTasks } from '$lib/repositories/task.repository';
	import { findAll as findAllLabels } from '$lib/repositories/label.repository';
	import { getAppVersion } from '$lib/updater/update.service';
	import AppearanceSection from '../../components/settings/AppearanceSection.svelte';
	import SidebarSection from '../../components/settings/SidebarSection.svelte';
	import TasksSection from '../../components/settings/TasksSection.svelte';
	import DataSection from '../../components/settings/DataSection.svelte';
	import BackupSection from '../../components/settings/BackupSection.svelte';
	import ShortcutsSection from '../../components/settings/ShortcutsSection.svelte';
	import WorkspaceSection from '../../components/settings/WorkspaceSection.svelte';
	import LiveSection from '../../components/settings/LiveSection.svelte';
	import CliSection from '../../components/settings/CliSection.svelte';
	import AboutSection from '../../components/settings/AboutSection.svelte';

	let settings = $state<Settings>(getSettings());
	let stats = $state({ projects: 0, tasks: 0, done: 0, labels: 0 });
	let appVersion = $state('');
	let activeTab = $state('appearance');
	let searchQuery = $state('');

	type SettingsSearchItem = {
		tab: string;
		label: string;
		description: string;
		keywords: (string | number)[];
		switchValue?: boolean;
		// for enum settings: editing from the search result updates the setting
		key?: keyof Settings;
		options?: { value: string; label: string }[];
	};

	// searchable index of every settings row, with current values as keywords
	// so searching "dark", "17890" or "board" lands on the right section
	let searchIndex = $derived.by((): SettingsSearchItem[] => {
		const s = settings;
		return [
			{
				tab: 'appearance',
				label: 'Theme',
				description: 'Choose how tack looks to you',
				keywords: ['dark', 'light', 'system', s.theme],
				key: 'theme',
				options: [
					{ value: 'dark', label: 'Dark' },
					{ value: 'light', label: 'Light' },
					{ value: 'system', label: 'System' }
				]
			},
			{
				tab: 'appearance',
				label: 'Collapse sidebar',
				description: 'Hide sidebar labels and project list',
				keywords: ['collapsed', 'compact', 'narrow'],
				switchValue: s.sidebarCollapsed
			},
			{
				tab: 'appearance',
				label: 'Default view',
				description: 'Which view to open by default',
				keywords: ['list', 'board', 'calendar', s.defaultViewMode],
				key: 'defaultViewMode',
				options: [
					{ value: 'list', label: 'List' },
					{ value: 'board', label: 'Board' },
					{ value: 'calendar', label: 'Calendar' }
				]
			},
			{
				tab: 'sidebar',
				label: 'Sidebar items',
				description: 'Drag to reorder, toggle to show or hide',
				keywords: [
					'pinned',
					'today',
					'upcoming',
					'overdue',
					'status',
					'priority',
					'quick stats',
					'visibility',
					'reorder'
				]
			},
			{
				tab: 'tasks',
				label: 'Default status',
				description: 'Status assigned to new tasks',
				keywords: ['todo', 'in progress', 'initial', s.defaultStatus],
				key: 'defaultStatus',
				options: [
					{ value: 'todo', label: 'Todo' },
					{ value: 'in_progress', label: 'In progress' }
				]
			},
			{
				tab: 'tasks',
				label: 'Default priority',
				description: 'Priority assigned to new tasks',
				keywords: ['urgent', 'high', 'medium', 'low', 'none', s.defaultPriority],
				key: 'defaultPriority',
				options: [
					{ value: '0', label: 'No priority' },
					{ value: '1', label: 'Urgent' },
					{ value: '2', label: 'High' },
					{ value: '3', label: 'Medium' },
					{ value: '4', label: 'Low' }
				]
			},
			{
				tab: 'tasks',
				label: 'Due soon threshold',
				description: 'Days ahead to flag tasks as due soon',
				keywords: ['days', 'threshold', 'due', s.dueSoonThreshold]
			},
			{
				tab: 'tasks',
				label: 'Task id padding',
				description: 'Zero-pad task numbers (0 = no padding, 3 = TSK-001)',
				keywords: ['number', 'align', 'digits', 'prefix', s.prefixPadding]
			},
			{
				tab: 'data',
				label: 'Export data',
				description: 'Save all projects, tasks and labels as a JSON file',
				keywords: ['json', 'save', 'file']
			},
			{
				tab: 'data',
				label: 'Import data',
				description: 'Load projects, tasks and labels from a JSON file',
				keywords: ['json', 'load', 'file', 'restore']
			},
			{
				tab: 'data',
				label: 'Delete all data',
				description: 'Remove everything and start over',
				keywords: ['reset', 'wipe', 'clear', 'danger']
			},
			{
				tab: 'backup',
				label: 'Local backups',
				description: 'Snapshots stored on this device',
				keywords: ['snapshot', 'restore', 'delete']
			},
			{
				tab: 'backup',
				label: 'Backup schedule',
				description: 'How often a new snapshot is taken',
				keywords: ['interval', 'hours', 'automatic', s.backupIntervalHours]
			},
			{
				tab: 'backup',
				label: 'Backups to keep',
				description: 'Older snapshots are removed automatically',
				keywords: ['retention', 'count', 'keep', s.backupKeepCount]
			},
			{
				tab: 'shortcuts',
				label: 'Keyboard shortcuts',
				description: 'Change the key combinations for actions',
				keywords: ['hotkeys', 'keys', 'command', 'key bindings']
			},
			{
				tab: 'live',
				label: 'Live server',
				description: 'Share your workspace in a browser on this device or your local network',
				keywords: ['browser', 'site', 'share', 'local network', 'server', 'live', s.livePort],
				switchValue: s.liveEnabled
			},
			{
				tab: 'live',
				label: 'Port',
				description: 'Where the server listens on this device',
				keywords: ['network', 'address', 'http', s.livePort]
			},
			{
				tab: 'workspace',
				label: 'Workspace stats',
				description: 'Projects, tasks, completed and completion rate',
				keywords: ['count', 'statistics', 'overview']
			},
			{
				tab: 'workspace',
				label: 'Install CLI',
				description: 'Add the tack command to your PATH',
				keywords: ['terminal', 'command line', 'path', 'cli']
			},
			{
				tab: 'workspace',
				label: 'Version',
				description: 'Version, updates and links',
				keywords: ['about', 'update', 'license', 'github', 'information']
			}
		];
	});

	const tabLabels: Record<string, string> = {
		appearance: 'Appearance',
		sidebar: 'Sidebar',
		tasks: 'Tasks',
		data: 'Data',
		backup: 'Backup',
		shortcuts: 'Shortcuts',
		live: 'Live',
		workspace: 'Workspace'
	};

	let searching = $derived(searchQuery.trim().length > 0);

	let searchResults = $derived.by(() => {
		const q = searchQuery.trim().toLowerCase();
		if (!q) return [];
		return searchIndex.filter((item) =>
			[item.label, item.description, ...item.keywords].some((text) =>
				String(text).toLowerCase().includes(q)
			)
		);
	});

	let resultTabs = $derived([...new Set(searchResults.map((r) => r.tab))]);

	let highlightTimer: ReturnType<typeof setTimeout> | undefined;

	function jumpToSettings(tab: string, label: string) {
		activeTab = tab;
		searchQuery = '';
		// wait for the tab content to render, then flash the matching row
		requestAnimationFrame(() => {
			requestAnimationFrame(() => flashSettingsLabel(label));
		});
	}

	// flash the settings row whose title matches the search result for 3s
	function flashSettingsLabel(label: string) {
		const labelEl = [...document.querySelectorAll('p')].find(
			(p) =>
				p.classList.contains('text-[13px]') &&
				p.classList.contains('font-medium') &&
				p.textContent?.trim() === label &&
				p.checkVisibility()
		);
		if (!labelEl) return;
		// nearest justify-between ancestor is the actual settings row
		const row = labelEl.closest('.justify-between') ?? labelEl.parentElement?.parentElement;
		if (!row) return;
		row.classList.add('settings-highlight-row');
		if (highlightTimer) clearTimeout(highlightTimer);
		highlightTimer = setTimeout(() => row.classList.remove('settings-highlight-row'), 3000);
	}

	function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
		settings = setSettings({ [key]: value });
		if (key === 'theme') applyTheme(value as Theme);
	}

	async function loadStats() {
		try {
			const [p, t, l] = await Promise.all([findAllProjects(), findAllTasks(), findAllLabels()]);
			stats = {
				projects: p.length,
				tasks: t.length,
				done: t.filter((task) => task.status === 'done').length,
				labels: l.length
			};
		} catch {
			// ignore
		}
	}

	onMount(() => {
		void loadStats();
		void getAppVersion().then((v) => (appVersion = v));

		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		const unlisten = onDbChanged(() => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void loadStats(), 200);
		});

		// sync when sidebar items reordered from sidebar itself
		const onSettingsChanged = () => {
			settings = getSettings();
		};
		window.addEventListener('settings-changed', onSettingsChanged);

		return () => {
			unlisten();
			window.removeEventListener('settings-changed', onSettingsChanged);
		};
	});
</script>

<section class="flex h-full flex-col">
	<!-- header -->
	<header class="flex items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-6 sm:py-4">
		<div class="min-w-0">
			<h1 class="text-base font-semibold tracking-tight sm:text-lg">Settings</h1>
			<p class="truncate text-xs text-muted-foreground sm:text-sm">
				Manage your workspace and preferences
			</p>
		</div>
		<Button variant="ghost" size="sm" href="/" class="shrink-0">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
				/></svg
			>
			Back
		</Button>
	</header>

	<!-- content -->
	<div class="flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
		<div class="mx-auto max-w-2xl">
			<!-- search -->
			<div class="relative mb-5">
				<svg
					class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground/50"
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					><path
						fill="currentColor"
						d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
					/></svg
				>
				<Input
					bind:value={searchQuery}
					placeholder="Search settings..."
					class="h-8 w-full rounded-lg border border-input bg-transparent pr-8 pl-8 text-[13px] text-foreground transition-all outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
				/>
				{#if searchQuery}
					<Button
						variant="ghost"
						size="icon-xs"
						onclick={() => (searchQuery = '')}
						class="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground/50 transition-colors hover:text-foreground"
						aria-label="Clear search"
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
							/></svg
						>
					</Button>
				{/if}
			</div>

			{#if searching}
				<!-- search results -->
				{#if searchResults.length === 0}
					<div class="flex flex-col items-center gap-3 py-20 text-center">
						<div class="flex size-10 items-center justify-center rounded-xl bg-muted/50">
							<svg
								class="text-muted-foreground"
								width="16"
								height="16"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
								/></svg
							>
						</div>
						<div>
							<p class="text-[13px] font-medium">No settings match</p>
							<p class="text-xs text-muted-foreground">Try a different word, or clear the search</p>
						</div>
						<Button variant="outline" size="sm" onclick={() => (searchQuery = '')}>
							clear search
						</Button>
					</div>
				{:else}
					<div class="space-y-5">
						{#each resultTabs as tab (tab)}
							<div>
								<div class="mb-1.5 flex items-center gap-2 px-1">
									<span class="text-[11px] font-medium text-muted-foreground"
										>{tabLabels[tab] ?? tab}</span
									>
									<span
										class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
										>{searchResults.filter((r) => r.tab === tab).length}</span
									>
								</div>
								<Card.Root size="sm" class="!gap-0 !py-0">
									{#each searchResults.filter((r) => r.tab === tab) as item, i (item.label)}
										<div
											class="flex w-full items-center justify-between gap-3 px-4 py-2.5 transition-colors hover:bg-muted/40 {i >
											0
												? 'border-t border-border/60'
												: ''}"
										>
											<button
												type="button"
												onclick={() => jumpToSettings(item.tab, item.label)}
												class="min-w-0 flex-1 text-left"
											>
												<p class="text-[13px] font-medium">{item.label}</p>
												<p class="truncate text-xs text-muted-foreground">
													{item.description}
												</p>
											</button>
											{#if item.switchValue !== undefined}
												<Switch
													checked={item.switchValue}
													tabindex={-1}
													class="pointer-events-none"
												/>
											{:else if item.key && item.options}
												<Select.Root
													type="single"
													value={String(settings[item.key!])}
													onValueChange={(v) => updateSetting(item.key!, v as never)}
												>
													<Select.Trigger size="sm" class="w-28">
														{item.options.find((o) => o.value === String(settings[item.key!]))
															?.label ?? String(settings[item.key!])}
													</Select.Trigger>
													<Select.Content>
														{#each item.options as opt (opt.value)}
															<Select.Item value={opt.value} label={opt.label}
																>{opt.label}</Select.Item
															>
														{/each}
													</Select.Content>
												</Select.Root>
											{/if}
										</div>
									{/each}
								</Card.Root>
							</div>
						{/each}
					</div>
				{/if}
			{:else}
				<Tabs.Root bind:value={activeTab} class="w-full">
					<div class="overflow-x-auto pb-0.5">
						<Tabs.List class="flex w-max gap-1 rounded-lg bg-muted/50 p-1 sm:w-full">
							<Tabs.Trigger value="appearance" class="shrink-0 flex-1 whitespace-nowrap">Appearance</Tabs.Trigger>
							<Tabs.Trigger value="sidebar" class="shrink-0 flex-1 whitespace-nowrap">Sidebar</Tabs.Trigger>
							<Tabs.Trigger value="tasks" class="shrink-0 flex-1 whitespace-nowrap">Tasks</Tabs.Trigger>
							<Tabs.Trigger value="data" class="shrink-0 flex-1 whitespace-nowrap">Data</Tabs.Trigger>
							<Tabs.Trigger value="backup" class="shrink-0 flex-1 whitespace-nowrap">Backup</Tabs.Trigger>
							<Tabs.Trigger value="shortcuts" class="shrink-0 flex-1 whitespace-nowrap">Shortcuts</Tabs.Trigger>
							<Tabs.Trigger value="live" class="shrink-0 flex-1 whitespace-nowrap">Live</Tabs.Trigger>
							<Tabs.Trigger value="workspace" class="shrink-0 flex-1 whitespace-nowrap">Workspace</Tabs.Trigger>
						</Tabs.List>
					</div>

					<Tabs.Content value="appearance" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<AppearanceSection {settings} update={updateSetting} />
					</Tabs.Content>

					<Tabs.Content value="sidebar" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<SidebarSection {settings} update={updateSetting} />
					</Tabs.Content>

					<Tabs.Content value="tasks" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<TasksSection {settings} update={updateSetting} />
					</Tabs.Content>

					<Tabs.Content value="data" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<DataSection />
					</Tabs.Content>

					<Tabs.Content value="backup" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<BackupSection />
					</Tabs.Content>

					<Tabs.Content value="shortcuts" class="mt-4 space-y-4 sm:mt-6">
						<ShortcutsSection />
					</Tabs.Content>

					<Tabs.Content value="live" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<LiveSection {settings} update={updateSetting} />
					</Tabs.Content>

					<Tabs.Content value="workspace" class="mt-4 space-y-5 sm:mt-6 sm:space-y-6">
						<WorkspaceSection {stats} />
						<CliSection />
						<Separator class="bg-border/40" />
						<AboutSection {appVersion} />
					</Tabs.Content>
				</Tabs.Root>

				<footer class="mt-8 flex items-center justify-between border-t border-border/60 pt-4">
					<p class="text-xs text-muted-foreground">Tack</p>
					<p class="text-xs text-muted-foreground">
						{#if appVersion}Version {appVersion}{:else}Version ...{/if}
					</p>
				</footer>
			{/if}
		</div>
	</div>
</section>
