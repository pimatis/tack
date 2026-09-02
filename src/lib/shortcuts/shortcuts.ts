export type ShortcutKey = {
	key: string;
	mod?: 'meta' | 'ctrl' | 'metaOrCtrl';
	shift?: boolean;
	alt?: boolean;
};

export type ShortcutDefinition = {
	id: string;
	label: string;
	keys: ShortcutKey[];
};

const metaOrCtrl = 'metaOrCtrl' as const;

export const SHORTCUTS: ShortcutDefinition[] = [
	{
		id: 'command-palette',
		label: 'Open command palette',
		keys: [{ key: 'k', mod: metaOrCtrl }]
	},
	{
		id: 'new-task',
		label: 'Create new task',
		keys: [{ key: 'c' }]
	},
	{
		id: 'new-project',
		label: 'Create new project',
		keys: [{ key: 'n' }]
	},
	{
		id: 'toggle-sidebar',
		label: 'Toggle sidebar',
		keys: [{ key: 'b', mod: metaOrCtrl }]
	},
	{
		id: 'toggle-view',
		label: 'Toggle list / board view',
		keys: [{ key: '\\', mod: metaOrCtrl }]
	},
	{
		id: 'select-all',
		label: 'Select all tasks',
		keys: [{ key: 'a', mod: metaOrCtrl }]
	},
	{
		id: 'close',
		label: 'Close dialog / panel',
		keys: [{ key: 'Escape' }]
	},
	{
		id: 'save-task',
		label: 'Save task',
		keys: [{ key: 'Enter', mod: metaOrCtrl }]
	}
];

function keyLabel(key: string): string {
	if (key === 'Escape') return 'Esc';
	if (key === ' ') return 'Space';
	if (key.length === 1) return key.toUpperCase();
	return key;
}

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

function keyModifierLabel(mod?: ShortcutKey['mod']): string | null {
	if (mod === 'meta') return isMac ? '⌘' : 'Win';
	if (mod === 'ctrl') return isMac ? '⌃' : 'Ctrl';
	if (mod === 'metaOrCtrl') return isMac ? '⌘' : 'Ctrl';
	return null;
}

export function keyComboLabel(key: ShortcutKey): string {
	const parts: string[] = [];
	const mod = keyModifierLabel(key.mod);
	if (mod) parts.push(mod);
	if (key.alt) parts.push(isMac ? '⌥' : 'Alt');
	if (key.shift) parts.push(isMac ? '⇧' : 'Shift');
	parts.push(keyLabel(key.key));
	return parts.join(' ');
}

export function shortcutIdLabel(id: string): string {
	return SHORTCUTS.find((s) => s.id === id)?.label ?? id;
}

export function combosEqual(a: ShortcutKey, b: ShortcutKey): boolean {
	return (
		a.key === b.key &&
		(a.mod ?? null) === (b.mod ?? null) &&
		(a.shift ?? false) === (b.shift ?? false) &&
		(a.alt ?? false) === (b.alt ?? false)
	);
}

// build a shortcut from a keyboard event while recording; null while only modifiers are pressed
export function comboFromEvent(e: KeyboardEvent): ShortcutKey | null {
	if (e.key === 'Process' || ['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return null;
	const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
	let mod: ShortcutKey['mod'] | undefined;
	if (e.metaKey && e.ctrlKey) mod = 'metaOrCtrl';
	else if (e.metaKey) mod = 'meta';
	else if (e.ctrlKey) mod = 'ctrl';
	return { key, mod, shift: e.shiftKey || undefined, alt: e.altKey || undefined };
}
