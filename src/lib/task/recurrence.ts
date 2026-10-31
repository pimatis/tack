// recurring task schedules. the rule stays one short string so the app, the
// cli and the live server read the same column without separate storage:
// daily, weekly:<iso weekday 1-7> or monthly:<day 1-31>

export type RecurrenceFrequency = 'daily' | 'weekly' | 'monthly';

export type RecurrenceRule = {
	frequency: RecurrenceFrequency;
	// iso weekday, monday = 1 ... sunday = 7
	weekday?: number;
	// day of the month, 1..31; clamped to the month's last day
	monthDay?: number;
};

export const WEEKDAYS = [
	{ value: 1, label: 'Monday' },
	{ value: 2, label: 'Tuesday' },
	{ value: 3, label: 'Wednesday' },
	{ value: 4, label: 'Thursday' },
	{ value: 5, label: 'Friday' },
	{ value: 6, label: 'Saturday' },
	{ value: 7, label: 'Sunday' }
] as const;

const DAY_MS = 24 * 60 * 60 * 1000;

export function parseRecurrence(rule: string | null | undefined): RecurrenceRule | null {
	if (!rule) return null;
	const [frequency, arg] = rule.split(':');
	if (frequency === 'daily') return { frequency };
	if (frequency === 'weekly') {
		const weekday = Number(arg);
		if (Number.isInteger(weekday) && weekday >= 1 && weekday <= 7) {
			return { frequency, weekday };
		}
	}
	if (frequency === 'monthly') {
		const monthDay = Number(arg);
		if (Number.isInteger(monthDay) && monthDay >= 1 && monthDay <= 31) {
			return { frequency, monthDay };
		}
	}
	return null;
}

export function serializeRecurrence(rule: RecurrenceRule): string {
	if (rule.frequency === 'weekly') return `weekly:${rule.weekday ?? 1}`;
	if (rule.frequency === 'monthly') return `monthly:${rule.monthDay ?? 1}`;
	return 'daily';
}

// human label shared by the task row, the detail panel and the picker
export function recurrenceLabel(rule: string | null | undefined): string | null {
	const parsed = parseRecurrence(rule);
	if (!parsed) return null;
	if (parsed.frequency === 'daily') return 'Every day';
	if (parsed.frequency === 'weekly') {
		const day = WEEKDAYS.find((d) => d.value === parsed.weekday);
		return `Every ${day?.label ?? 'week'}`;
	}
	return `Every month on the ${ordinal(parsed.monthDay ?? 1)}`;
}

function ordinal(n: number): string {
	const mod100 = n % 100;
	if (mod100 >= 11 && mod100 <= 13) return `${n}th`;
	switch (n % 10) {
		case 1:
			return `${n}st`;
		case 2:
			return `${n}nd`;
		case 3:
			return `${n}rd`;
		default:
			return `${n}th`;
	}
}

export function todayIso(): string {
	return isoDate(new Date());
}

// the next date the rule fires after the task's due date; a stale occurrence
// starts counting from today so the series never lands in the past
export function nextOccurrence(
	rule: string | null | undefined,
	dueDate: string | null | undefined,
	today = todayIso()
): { dueDate: string; deltaDays: number } | null {
	const parsed = parseRecurrence(rule);
	if (!parsed) return null;

	const base = dueDate && dueDate > today ? parseIsoDate(dueDate) : parseIsoDate(today);
	let next: Date;

	if (parsed.frequency === 'daily') {
		next = addDays(base, 1);
	} else if (parsed.frequency === 'weekly') {
		next = addDays(base, 1);
		while (isoWeekday(next) !== parsed.weekday) next = addDays(next, 1);
	} else {
		const month = new Date(base.getFullYear(), base.getMonth() + 1, 1);
		const monthDay = Math.min(parsed.monthDay ?? 1, daysInMonth(month));
		next = new Date(month.getFullYear(), month.getMonth(), monthDay);
	}

	const previous = dueDate ? parseIsoDate(dueDate) : parseIsoDate(today);
	return {
		dueDate: isoDate(next),
		deltaDays: Math.round((next.getTime() - previous.getTime()) / DAY_MS)
	};
}

export function addDaysIso(value: string, days: number): string {
	return isoDate(addDays(parseIsoDate(value), days));
}

// reminders are utc instants, so the shift keeps the clock time and moves the
// calendar day with the occurrence
export function addDaysTimestamp(iso: string, days: number): string {
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return iso;
	date.setDate(date.getDate() + days);
	return date.toISOString();
}

// due dates are plain calendar days; parse them in local time so a date never
// drifts a day across timezones
export function parseIsoDate(value: string): Date {
	const [year, month, day] = value.split('-').map(Number);
	return new Date(year, month - 1, day);
}

export function isoDate(date: Date): string {
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

function addDays(date: Date, days: number): Date {
	const next = new Date(date);
	next.setDate(next.getDate() + days);
	return next;
}

function isoWeekday(date: Date): number {
	return date.getDay() === 0 ? 7 : date.getDay();
}

function daysInMonth(date: Date): number {
	return new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
}
