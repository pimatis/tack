<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import Sidebar from '../components/Sidebar.svelte';
	import CommandPalette from '../components/CommandPalette.svelte';
	import Shortcuts from '../components/Shortcuts.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
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
	import { isLiveAuthRequired } from '$lib/live/auth.service';
	import LiveAuthDialog from '../components/LiveAuthDialog.svelte';
	import NotesView from '../components/notes/NotesView.svelte';
	import NotesSearchDialog from '../components/notes/NotesSearchDialog.svelte';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { isTauri, onDbChanged, onNotesChanged } from '$lib/db/client';
	import { invoke } from '@tauri-apps/api/core';
	import { afterNavigate } from '$app/navigation';
	import { Toaster } from '$lib/components/ui/sonner';

	const { children } = $props();

	// notes view only replaces the home route; /trash, /settings etc. still render
	let pathname = $state('/');
	afterNavigate(() => (pathname = window.location.pathname));

	// load persisted settings before any child component reads them
	let settings = $state(initSettings());

	// remember the tasks/notes tab across reloads; the browser reload drops
	// the in-memory state, so the live site would otherwise open on tasks
	$effect(() => {
		notesState.persistActiveTab(notesState.activeTab);
	});

	// narrow viewports (tablets/phones) switch the sidebar to drawer mode;
	// the drawer open state lives here so the hamburger (main column) can drive it
	let isNarrow = $state(false);
	let mobileOpen = $state(false);

	// live share is password-protected: hide the whole app (including any
	// data fetched before the server started asking for the password)
	let authLocked = $state(false);

	onMount(() => {
		// reveal the window as soon as the shell is mounted; direct ipc works
		// even while the window is hidden (rAF does not fire off-screen)
		invoke('show_window').catch(() => {});

		const lock = () => (authLocked = true);
		if (isLiveAuthRequired()) authLocked = true;
		window.addEventListener('live-auth-required', lock);

		// keep in sync with cli changes (tack settings set) while the app is running
		void loadSettingsFromDb();
		const syncInterval = window.setInterval(() => void loadSettingsFromDb(), 30000);
		const stopBackups = startBackupScheduler();
		const stopLive = startLiveManager();

		const narrowQuery = window.matchMedia('(max-width: 1023px)');
		const applyNarrow = () => (isNarrow = narrowQuery.matches);
		applyNarrow();
		narrowQuery.addEventListener('change', applyNarrow);

		const onSettingsChanged = () => {
			settings = getSettings();
		};
		window.addEventListener('settings-changed', onSettingsChanged);

		// notes: restore folder + list notes once
		void notesState.init();

		// notes stay current no matter which tab is open: external/cli edits
		// arrive as notes-changed on desktop and as db-changed over the live
		// sse stream in the browser
		let notesSyncTimer: ReturnType<typeof setTimeout> | undefined;
		const requestNotesSync = () => {
			clearTimeout(notesSyncTimer);
			notesSyncTimer = setTimeout(() => void notesState.syncExternal(), 600);
		};
		const unlistenNotes = onNotesChanged(requestNotesSync);
		const unlistenNotesDb = onDbChanged(() => void notesState.reloadPins());

		// the live editor debounces saves; flush the last keystrokes before the
		// tab is hidden or closed so the desktop and cli see them
		const flushNotes = () => notesState.flushPendingSave();
		const onVisibilityChange = () => {
			if (document.visibilityState === 'hidden') notesState.flushPendingSave();
		};
		window.addEventListener('pagehide', flushNotes);
		document.addEventListener('visibilitychange', onVisibilityChange);

		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const handleThemeChange = () => {
			if (getSettings().theme === 'system') applyTheme('system');
		};
		mediaQuery.addEventListener('change', handleThemeChange);
		return () => {
			window.clearInterval(syncInterval);
			stopBackups();
			stopLive();
			clearTimeout(notesSyncTimer);
			unlistenNotes();
			unlistenNotesDb();
			window.removeEventListener('pagehide', flushNotes);
			document.removeEventListener('visibilitychange', onVisibilityChange);
			narrowQuery.removeEventListener('change', applyNarrow);
			mediaQuery.removeEventListener('change', handleThemeChange);
			window.removeEventListener('settings-changed', onSettingsChanged);
			window.removeEventListener('live-auth-required', lock);
		};
	});

	function toggleSidebar() {
		if (isNarrow) return;
		settings = setSettings({ sidebarCollapsed: !settings.sidebarCollapsed });
	}
</script>

{#if authLocked}
	<!-- locked: no app shell, no previously fetched data — just the gate -->
	<div class="flex h-screen w-full items-center justify-center bg-background">
		<LiveAuthDialog />
	</div>
{:else}
	<Tooltip.Provider>
		<Shortcuts>
			<div class="flex h-screen w-full overflow-hidden bg-background text-foreground">
				<Sidebar
					settings={{ ...settings, sidebarCollapsed: settings.sidebarCollapsed || isNarrow }}
					narrow={isNarrow}
					{toggleSidebar}
					bind:mobileOpen
				/>
				<main class="flex flex-1 flex-col overflow-hidden">
					<div
						class="flex shrink-0 items-center {isNarrow ? 'h-12' : 'h-7'} {isTauri()
							? 'pl-[78px]'
							: 'pl-2'}"
						data-tauri-drag-region
					>
						{#if isNarrow && !mobileOpen}
							<Button
								variant="ghost"
								size="icon-sm"
								data-tauri-drag-region-ignore
								class="!h-6 !w-6 text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
								onclick={() => (mobileOpen = true)}
								aria-label="Open sidebar"
							>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="20"
									height="20"
									viewBox="0 0 24 24"
									class="size-5"
									><path
										fill="currentColor"
										d="M20 17.5a1.5 1.5 0 0 1 .144 2.993L20 20.5H4a1.5 1.5 0 0 1-.144-2.993L4 17.5zm0-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 0 1 0-3zm0-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 1 1 0-3z"
									/></svg
								>
							</Button>
						{/if}
					</div>
					<div class="m-2 min-h-0 flex-1 overflow-auto rounded-xl border border-border bg-card">
						{#if notesState.activeTab === 'notes' && pathname === '/'}
							<NotesView />
						{:else}
							{@render children()}
						{/if}
					</div>
				</main>
			</div>
			<CommandPalette />
			<NotesSearchDialog />
		</Shortcuts>
		<Toaster position="bottom-right" />
	</Tooltip.Provider>
{/if}
