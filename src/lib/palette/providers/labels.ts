import { labelColorMap } from '$lib/types/label';
import { goHomeThenDispatch, matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';
import type { Label } from '$lib/types/label';

function toItem(label: Label): PaletteItem {
	return {
		id: label.id,
		label: label.name,
		dot: labelColorMap[label.color].dot,
		run: () => goHomeThenDispatch('filter-by-label', label.id)
	};
}

export const labelsProvider: PaletteProvider = {
	id: 'labels',
	heading: 'Labels',
	search(query, ctx) {
		return ctx.labels.filter((label) => matches(label.name, query)).map(toItem);
	},
	resolve(itemId, ctx) {
		const label = ctx.labels.find((l) => l.id === itemId);
		if (!label) return null;
		return toItem(label);
	}
};
