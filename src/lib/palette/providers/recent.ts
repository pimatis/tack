import { readRecents } from '../recents';
import { matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';

// replays stored provider+item refs through each provider's resolve(); the
// resolved item keeps its original providerId so selecting it records the
// real group again instead of "recent"
export const recentProvider: PaletteProvider = {
	id: 'recent',
	heading: 'Recent',
	search(query, ctx) {
		if (!ctx.resolve) return [];
		return readRecents()
			.map((ref) => {
				const item = ctx.resolve!(ref.provider, ref.item);
				if (item) item.providerId = ref.provider;
				return item;
			})
			.filter((item): item is PaletteItem => item !== null)
			.filter((item) => matches(item.label, query));
	}
};
