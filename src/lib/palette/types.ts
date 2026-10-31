import type { Task, TaskStatus } from '$lib/types/task';
import type { Project } from '$lib/types/project';
import type { Label } from '$lib/types/label';
import type { IconName } from '$lib/icons';

// data loaded once per palette open, shared by every provider
export type PaletteContext = {
	tasks: Task[];
	projects: Project[];
	labels: Label[];
	trash: Task[];
	// reload ctx after a mutation (sub-actions, restore) without closing
	refresh?: () => void;
	// rebuild an item by provider+item id; used by the recents provider
	resolve?: (providerId: string, itemId: string) => PaletteItem | null;
};

// extra key combos an item answers to while highlighted (cmd/ctrl+enter, alt+enter)
export type PaletteSubAction = {
	combo: 'mod-enter' | 'alt-enter';
	// short keycap shown next to the item, e.g. "⌘⏎"
	hint: string;
	label: string;
	run: () => void | Promise<void>;
};

export type PaletteItem = {
	id: string;
	label: string;
	icon?: IconName;
	// renders StatusIcon instead of a semantic icon (task and status rows)
	status?: TaskStatus;
	// small color dot instead of an icon (labels)
	dot?: string;
	// right-aligned muted text (issue id, project prefix, key hint, "Active")
	hint?: string;
	// extra words the item should match on, never rendered
	keywords?: string;
	// applied while the item is highlighted; reverted on close unless selected
	preview?: () => void;
	actions?: PaletteSubAction[];
	// set by the recents provider so re-selecting records the original group
	providerId?: string;
	// items that can never be rebuilt by resolve() opt out of recents
	noRecent?: boolean;
	run: () => void | Promise<void>;
};

export type PaletteProvider = {
	id: string;
	heading: string;
	// query is the raw trimmed input (after any scope prefix); scoped is true
	// when the query came through a prefix like ">" or "#"
	search: (
		query: string,
		ctx: PaletteContext,
		scoped: boolean
	) => PaletteItem[] | Promise<PaletteItem[]>;
	// rebuild one item by id for the recents list; null when it no longer exists
	resolve?: (itemId: string, ctx: PaletteContext) => PaletteItem | null;
};
