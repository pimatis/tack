export type ShortcutKey = {
	key: string;
	mod?: 'meta' | 'ctrl' | 'metaOrCtrl';
};

export type ShortcutDefinition = {
	id: string;
	label: string;
	keys: ShortcutKey[];
};

const meta = 'meta' as const;
const ctrl = 'ctrl' as const;
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
		id: 'toggle-view',
		label: 'Toggle list / board view',
		keys: [{ key: 'b', mod: metaOrCtrl }]
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

export function keyComboLabel(key: ShortcutKey): string {
	const base =
		key.key === 'Escape' ? 'Esc' : key.key.length === 1 ? key.key.toUpperCase() : key.key;
	if (key.mod === meta) return `⌘ ${base}`;
	if (key.mod === ctrl) return `Ctrl ${base}`;
	if (key.mod === metaOrCtrl) return `⌘ ${base}`;
	return base;
}

export function shortcutLabel(def: ShortcutDefinition): string {
	return def.keys.map(keyComboLabel).join(' / ');
}
