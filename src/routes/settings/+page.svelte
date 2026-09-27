<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Empty from '$lib/components/ui/empty/index.js';
	import * as InputGroup from '$lib/components/ui/input-group/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { getSettings, setSettings, applyTheme } from '$lib/stores/settings';
	import { colorPresets } from '$lib/theme/presets';
	import { onDbChanged } from '$lib/db/client';
	import type { Settings, Theme } from '$lib/types/settings';
	import { findAll as findAllProjects } from '$lib/repositories/project.repository';
	import { findAll as findAllTasks } from '$lib/repositories/task.repository';
	import { findAll as findAllLabels } from '$lib/repositories/label.repository';
	import { getAppVersion } from '$lib/updater/update.service';
	import AppearanceSection from '../../components/settings/AppearanceSection.svelte';
	import ThemesSection from '../../components/settings/ThemesSection.svelte';
	import SidebarSection from '../../components/settings/SidebarSection.svelte';
	import TasksSection from '../../components/settings/TasksSection.svelte';
	import DataSection from '../../components/settings/DataSection.svelte';
	import BackupSection from '../../components/settings/BackupSection.svelte';
	import ShortcutsSection from '../../components/settings/ShortcutsSection.svelte';
	import WorkspaceSection from '../../components/settings/WorkspaceSection.svelte';
	import LiveSection from '../../components/settings/LiveSection.svelte';
	import PermissionsSection from '../../components/settings/PermissionsSection.svelte';
	import CliSection from '../../components/settings/CliSection.svelte';
	import AboutSection from '../../components/settings/AboutSection.svelte';
	import MingcuteIcon from '../../components/notes/MingcuteIcon.svelte';

	type Section = {
		value: string;
		label: string;
		description: string;
		// mingcute icon name, resolved at runtime (see src/lib/notes/icons.ts)
		icon: string;
	};

	// single source of truth for the nav, the mobile picker and search labels
	const sections: Section[] = [
		{
			value: 'appearance',
			label: 'Appearance',
			description: 'Theme, color and layout basics',
			icon: 'palette-fill'
		},
		{
			value: 'themes',
			label: 'Themes',
			description: 'Pick a color palette for the whole app',
			icon: 'brush-fill'
		},
		{
			value: 'sidebar',
			label: 'Sidebar',
			description: 'Choose what shows in the sidebar',
			icon: 'layout-left-fill'
		},
		{ value: 'tasks', label: 'Tasks', description: 'Defaults for new tasks', icon: 'task-fill' },
		{
			value: 'data',
			label: 'Data',
			description: 'Import, export and reset your data',
			icon: 'storage-fill'
		},
		{
			value: 'backup',
			label: 'Backup',
			description: 'Automatic snapshots and restore points',
			icon: 'archive-fill'
		},
		{
			value: 'shortcuts',
			label: 'Shortcuts',
			description: 'Keyboard shortcuts for common actions',
			icon: 'keyboard-fill'
		},
		{
			value: 'live',
			label: 'Live',
			description: 'Share your workspace in a browser',
			icon: 'signal-fill'
		},
		{
			value: 'permissions',
			label: 'Permissions',
			description: 'System permissions tack can use',
			icon: 'notification-fill'
		},
		{
			value: 'workspace',
			label: 'Workspace',
			description: 'Stats, CLI and about tack',
			icon: 'dashboard-fill'
		}
	];

	const tabLabels: Record<string, string> = Object.fromEntries(
		sections.map((section) => [section.value, section.label])
	);

	let settings = $state<Settings>(getSettings());
	let stats = $state({ projects: 0, tasks: 0, done: 0, labels: 0 });
	let appVersion = $state('');
	let activeTab = $state('appearance');
	let searchQuery = $state('');

	let activeSection = $derived(
		sections.find((section) => section.value === activeTab) ?? sections[0]
	);

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
				tab: 'themes',
				label: 'Color theme',
				description: 'Pick a color palette for the whole app',
				keywords: [
					'preset',
					'palette',
					'color',
					...colorPresets.map((preset) => preset.label.toLowerCase()),
					s.themePreset
				],
				key: 'themePreset',
				options: colorPresets.map((preset) => ({ value: preset.id, label: preset.label }))
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
				tab: 'permissions',
				label: 'Notification permission',
				description: 'Allow tack to show system alerts for task reminders',
				keywords: ['permission', 'notifications', 'reminder', 'grant', 'allow', 'system', 'access']
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
	<header
		class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-6 sm:py-4"
	>
		<div class="min-w-0">
			<h1 class="text-base font-semibold tracking-tight sm:text-lg">Settings</h1>
			<p class="truncate text-xs text-muted-foreground sm:text-sm">
				Manage your workspace and preferences
			</p>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			<InputGroup.Root
				class="h-8! w-40 rounded-lg! border-input/30 bg-input/30 shadow-none! *:data-[slot=input-group-addon]:pl-2! sm:w-56"
			>
				<InputGroup.Input
					bind:value={searchQuery}
					placeholder="Search settings"
					class="text-[13px]"
				/>
				<InputGroup.Addon><MingcuteIcon icon="search-fill" size={15} /></InputGroup.Addon>
				{#if searchQuery}
					<InputGroup.Addon class="pr-1">
						<Button
							variant="ghost"
							size="icon-xs"
							onclick={() => (searchQuery = '')}
							aria-label="Clear search"
							class="text-muted-foreground hover:text-foreground"
						>
							<MingcuteIcon icon="close-fill" size={14} />
						</Button>
					</InputGroup.Addon>
				{/if}
			</InputGroup.Root>
			<Button variant="ghost" size="sm" href="/">
				<MingcuteIcon icon="left-fill" />
				Back
			</Button>
		</div>
	</header>

	<!-- body -->
	<div class="flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
		<div class="mx-auto w-full max-w-[62rem]">
			{#if searching}
				<!-- search results -->
				{#if searchResults.length === 0}
					<Empty.Root class="border border-dashed border-border/70">
						<Empty.Header>
							<Empty.Media variant="icon">
								<MingcuteIcon icon="search-fill" size={24} />
							</Empty.Media>
							<Empty.Title>No settings match</Empty.Title>
							<Empty.Description>Try a different word, or clear the search.</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<Button variant="outline" size="sm" onclick={() => (searchQuery = '')}>
								Clear search
							</Button>
						</Empty.Content>
					</Empty.Root>
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
				<Tabs.Root
					bind:value={activeTab}
					orientation="vertical"
					class="flex flex-col gap-5 md:flex-row md:gap-8"
				>
					<!-- desktop nav -->
					<Tabs.List class="hidden gap-1 md:flex md:w-48 md:shrink-0 md:flex-col">
						{#each sections as section (section.value)}
							<Tabs.Trigger
								value={section.value}
								class="h-8 justify-start gap-2.5 px-2.5 text-[13px]"
							>
								<MingcuteIcon icon={section.icon} size={16} />
								<span class="truncate">{section.label}</span>
							</Tabs.Trigger>
						{/each}
					</Tabs.List>

					<div class="max-w-3xl min-w-0 flex-1">
						<!-- mobile picker -->
						<div class="mb-4 md:hidden">
							<Select.Root type="single" value={activeTab} onValueChange={(v) => (activeTab = v)}>
								<Select.Trigger class="w-full">
									<span data-slot="select-value">
										{#if activeSection}
											<MingcuteIcon icon={activeSection.icon} size={16} />
											<span>{activeSection.label}</span>
										{/if}
									</span>
								</Select.Trigger>
								<Select.Content>
									{#each sections as section (section.value)}
										<Select.Item value={section.value} label={section.label}>
											<MingcuteIcon icon={section.icon} size={16} />
											<span>{section.label}</span>
										</Select.Item>
									{/each}
								</Select.Content>
							</Select.Root>
						</div>

						<div class="mb-4">
							<h2 class="text-sm font-semibold tracking-tight">{activeSection.label}</h2>
							<p class="text-xs text-muted-foreground">{activeSection.description}</p>
						</div>

						<Tabs.Content value="appearance" class="flex flex-col gap-5">
							<AppearanceSection {settings} update={updateSetting} />
						</Tabs.Content>

						<Tabs.Content value="themes" class="flex flex-col gap-5">
							<ThemesSection {settings} update={updateSetting} />
						</Tabs.Content>

						<Tabs.Content value="sidebar" class="flex flex-col gap-5">
							<SidebarSection {settings} update={updateSetting} />
						</Tabs.Content>

						<Tabs.Content value="tasks" class="flex flex-col gap-5">
							<TasksSection {settings} update={updateSetting} />
						</Tabs.Content>

						<Tabs.Content value="data" class="flex flex-col gap-5">
							<DataSection />
						</Tabs.Content>

						<Tabs.Content value="backup" class="flex flex-col gap-5">
							<BackupSection />
						</Tabs.Content>

						<Tabs.Content value="shortcuts" class="flex flex-col gap-5">
							<ShortcutsSection />
						</Tabs.Content>

						<Tabs.Content value="live" class="flex flex-col gap-5">
							<LiveSection {settings} update={updateSetting} />
						</Tabs.Content>

						<Tabs.Content value="permissions" class="flex flex-col gap-5">
							<PermissionsSection />
						</Tabs.Content>

						<Tabs.Content value="workspace" class="flex flex-col gap-5">
							<WorkspaceSection {stats} />
							<CliSection />
							<Separator class="bg-border/40" />
							<AboutSection {appVersion} />
						</Tabs.Content>
					</div>
				</Tabs.Root>

				<!-- footer spans the nav + content width so the two ends line up -->
				<footer
					class="mt-8 flex max-w-[62rem] items-center justify-between border-t border-border/60 pt-4"
				>
					<p class="text-xs text-muted-foreground">Tack</p>
					<p class="text-xs text-muted-foreground">
						{#if appVersion}Version {appVersion}{:else}Version ...{/if}
					</p>
				</footer>
			{/if}
		</div>
	</div>
</section>
