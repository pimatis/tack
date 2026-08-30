export type LabelColor =
	'gray' | 'blue' | 'green' | 'amber' | 'red' | 'purple' | 'pink' | 'teal' | 'orange' | 'indigo';

export type Label = {
	id: string;
	name: string;
	color: LabelColor;
	createdAt: string;
};

export const labelColorMap: Record<LabelColor, { dot: string; badge: string }> = {
	gray: { dot: 'bg-muted-foreground/60', badge: 'bg-muted-foreground/10 text-muted-foreground' },
	blue: { dot: 'bg-blue-500', badge: 'bg-blue-500/10 text-blue-500' },
	green: { dot: 'bg-green-500', badge: 'bg-green-500/10 text-green-500' },
	amber: { dot: 'bg-amber-500', badge: 'bg-amber-500/10 text-amber-500' },
	red: { dot: 'bg-red-500', badge: 'bg-red-500/10 text-red-500' },
	purple: { dot: 'bg-purple-500', badge: 'bg-purple-500/10 text-purple-500' },
	pink: { dot: 'bg-pink-500', badge: 'bg-pink-500/10 text-pink-500' },
	teal: { dot: 'bg-teal-500', badge: 'bg-teal-500/10 text-teal-500' },
	orange: { dot: 'bg-orange-500', badge: 'bg-orange-500/10 text-orange-500' },
	indigo: { dot: 'bg-indigo-500', badge: 'bg-indigo-500/10 text-indigo-500' }
};

export const labelColorOptions: LabelColor[] = [
	'gray',
	'blue',
	'green',
	'amber',
	'red',
	'purple',
	'pink',
	'teal',
	'orange',
	'indigo'
];
