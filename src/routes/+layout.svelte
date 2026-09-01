<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import Sidebar from '../components/Sidebar.svelte';
	import CommandPalette from '../components/CommandPalette.svelte';
	import Shortcuts from '../components/Shortcuts.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import {
		initSettings,
		getSettings,
		setSettings,
		applyTheme,
		loadSettingsFromDb
	} from '$lib/stores/settings';
	import { startBackupScheduler } from '$lib/backup/backup.service';
	import { startLiveManager } from '$lib/live/live.service';

	const { children } = $props();

	// load persisted settings before any child component reads them
	let settings = $state(initSettings());

	// narrow viewports (phones/tablets viewing the live site) force the sidebar collapsed
	let isNarrow = $state(false);

	onMount(() => {
		// keep in sync with cli changes (tack settings set) while the app is running
		void loadSettingsFromDb();
		const syncInterval = window.setInterval(() => void loadSettingsFromDb(), 30000);
		const stopBackups = startBackupScheduler();
		const stopLive = startLiveManager();

		const narrowQuery = window.matchMedia('(max-width: 767px)');
		const applyNarrow = () => (isNarrow = narrowQuery.matches);
		applyNarrow();
		narrowQuery.addEventListener('change', applyNarrow);

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
			window.clearInterval(syncInterval);
			stopBackups();
			stopLive();
			narrowQuery.removeEventListener('change', applyNarrow);
			mediaQuery.removeEventListener('change', handleThemeChange);
			window.removeEventListener('settings-changed', onSettingsChanged);
		};
	});

	function toggleSidebar() {
		if (isNarrow) return;
		settings = setSettings({ sidebarCollapsed: !settings.sidebarCollapsed });
	}
</script>

<Tooltip.Provider>
	<Shortcuts>
		<div class="flex h-screen w-full overflow-hidden bg-background text-foreground">
			<Sidebar
				settings={{ ...settings, sidebarCollapsed: settings.sidebarCollapsed || isNarrow }}
				narrow={isNarrow}
				{toggleSidebar}
			/>
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
