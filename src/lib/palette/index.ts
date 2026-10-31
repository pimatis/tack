import type { PaletteProvider } from './types';
import { recentProvider } from './providers/recent';
import { actionsProvider } from './providers/actions';
import { tasksProvider } from './providers/tasks';
import { trashProvider } from './providers/trash';
import { projectsProvider } from './providers/projects';
import { labelsProvider } from './providers/labels';
import { filtersProvider } from './providers/filters';
import { themesProvider } from './providers/themes';
import { navigationProvider } from './providers/navigation';

// display order in the palette follows this array
export const paletteProviders: PaletteProvider[] = [
	recentProvider,
	actionsProvider,
	tasksProvider,
	trashProvider,
	projectsProvider,
	labelsProvider,
	filtersProvider,
	themesProvider,
	navigationProvider
];

export type { PaletteContext, PaletteItem, PaletteProvider } from './types';
