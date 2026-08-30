<script lang="ts">
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { getSettings, setSettings, applyTheme } from '$lib/stores/settings';
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
	import CliSection from '../../components/settings/CliSection.svelte';
	import AboutSection from '../../components/settings/AboutSection.svelte';

	let settings = $state<Settings>(getSettings());
	let stats = $state({ projects: 0, tasks: 0, done: 0, labels: 0 });
	let appVersion = $state('');

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
		const unlisten = listen('db-changed', () => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void loadStats(), 200);
		});

		// sync when sidebar items reordered from sidebar itself
		const onSettingsChanged = () => {
			settings = getSettings();
		};
		window.addEventListener('settings-changed', onSettingsChanged);

		return () => {
			unlisten.then((fn) => fn());
			window.removeEventListener('settings-changed', onSettingsChanged);
		};
	});
</script>

<section class="flex h-full flex-col">
	<!-- header -->
	<header class="flex items-center justify-between border-b border-border px-6 py-4">
		<div>
			<h1 class="text-lg font-semibold tracking-tight">Settings</h1>
			<p class="text-sm text-muted-foreground">Manage your workspace and preferences</p>
		</div>
		<Button variant="ghost" size="sm" href="/">
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
	<div class="flex-1 overflow-y-auto px-6 py-6">
		<div class="mx-auto max-w-2xl">
			<Tabs.Root value="appearance" class="w-full">
				<Tabs.List class="flex w-full gap-1 rounded-lg bg-muted/50 p-1">
					<Tabs.Trigger value="appearance" class="flex-1">Appearance</Tabs.Trigger>
					<Tabs.Trigger value="sidebar" class="flex-1">Sidebar</Tabs.Trigger>
					<Tabs.Trigger value="tasks" class="flex-1">Tasks</Tabs.Trigger>
					<Tabs.Trigger value="data" class="flex-1">Data</Tabs.Trigger>
					<Tabs.Trigger value="backup" class="flex-1">Backup</Tabs.Trigger>
					<Tabs.Trigger value="shortcuts" class="flex-1">Shortcuts</Tabs.Trigger>
					<Tabs.Trigger value="workspace" class="flex-1">Workspace</Tabs.Trigger>
				</Tabs.List>

				<Tabs.Content value="appearance" class="mt-6 space-y-6">
					<AppearanceSection {settings} update={updateSetting} />
				</Tabs.Content>

				<Tabs.Content value="sidebar" class="mt-6 space-y-6">
					<SidebarSection {settings} update={updateSetting} />
				</Tabs.Content>

				<Tabs.Content value="tasks" class="mt-6 space-y-6">
					<TasksSection {settings} update={updateSetting} />
				</Tabs.Content>

				<Tabs.Content value="data" class="mt-6 space-y-6">
					<DataSection />
				</Tabs.Content>

				<Tabs.Content value="backup" class="mt-6 space-y-6">
					<BackupSection />
				</Tabs.Content>

				<Tabs.Content value="shortcuts" class="mt-6 space-y-4">
					<ShortcutsSection />
				</Tabs.Content>

				<Tabs.Content value="workspace" class="mt-6 space-y-6">
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
		</div>
	</div>
</section>
