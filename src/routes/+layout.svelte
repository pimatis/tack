<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import Sidebar from '../components/Sidebar.svelte';
	import CommandPalette from '../components/CommandPalette.svelte';
	import Shortcuts from '../components/Shortcuts.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { initSettings, getSettings, setSettings, applyTheme } from '$lib/stores/settings';
	import { startBackupScheduler } from '$lib/backup/backup.service';

	const { children } = $props();

	let settings = $state(getSettings());

	onMount(() => {
		settings = initSettings();
		const stopBackups = startBackupScheduler();

		const onSettingsChanged = () => {
			settings = getSettings();
		};
		window.addEventListener('settings-changed', onSettingsChanged);

		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const handleThemeChange = () => {
			if (getSettings().theme === 'system') applyTheme('system');
		};
		mediaQuery.addEventListener('change', handleThemeChange);
		return () => {
			stopBackups();
			mediaQuery.removeEventListener('change', handleThemeChange);
			window.removeEventListener('settings-changed', onSettingsChanged);
		};
	});

	function toggleSidebar() {
		settings = setSettings({ sidebarCollapsed: !settings.sidebarCollapsed });
	}
</script>

<Tooltip.Provider>
	<Shortcuts>
		<div class="flex h-screen w-full overflow-hidden bg-background text-foreground">
			<Sidebar {settings} {toggleSidebar} />
			<main class="flex flex-1 flex-col overflow-hidden">
				<div class="h-7 shrink-0" data-tauri-drag-region></div>
				<div class="m-2 min-h-0 flex-1 overflow-auto rounded-xl border border-border bg-card">
					{@render children()}
				</div>
			</main>
		</div>
		<CommandPalette />
	</Shortcuts>
</Tooltip.Provider>
