import { isoDate } from '$lib/task/recurrence';
import langs from './langs.json';

// natural language quick-add parser, mirrored by the cli port in
// src-tauri/cli/src/quickadd.rs; both sides read the shared vocabulary in
// ./langs.json so a new language is a data change, not a code change.
// every language is active at once - users can mix ("yarın at 5pm").
// recognized tokens:
//   #key                       project (prefix or name, case-insensitive)
//   +name                      label (existing label name)
//   !urgent !high ... !p0..p4  priority (localized words in langs.json)
//   !pin !pinned               pin the task
//   ~90m ~2h ~1h30m ~90        estimate in minutes
//   today tomorrow YYYY-MM-DD <weekday>   due date; weekdays resolve to the
//                              next occurrence strictly after today
//   <next> <weekday|week|month>  due date pushed one step further (gelecek)
//   <in> <N><unit|d|w|m>       due date N units out ("in 3 days", "in 3d")
//   <on|by|due> <date>         due date
//   <end|until> <date>         end date
//   HH:MM, 3pm, 3:45pm, 18h30, <at> <time>   reminder; lands on the due date
//                              when one was given, else today (or tomorrow
//                              once the time has already passed)
//   daily weekly monthly weekly:N monthly:N   recurrence rule (storage format)
//   <every> <day|week|month|weekday|1-31>     same, natural form
// anything unrecognized stays in the title. quoted terms above ("next", "in",
// "every" ...) come from the merged vocabulary, not a fixed language.
export type ParsedQuickAdd = {
	title: string;
	projectKey: string | null;
	labelNames: string[];
	priority: number | null;
	pinned: boolean;
	dueDate: string | null;
	endDate: string | null;
	// utc iso instant for the reminder notification
	reminderAt: string | null;
	recurrenceRule: string | null;
	estimateMinutes: number | null;
};

type RuleKind = 'daily' | 'weekly' | 'monthly';

type LangVocab = {
	weekdays?: string[][];
	today?: string[];
	tomorrow?: string[];
	nextWeek?: string[];
	nextMonth?: string[];
	next?: string[];
	in?: string[];
	every?: string[];
	day?: string[];
	week?: string[];
	month?: string[];
	end?: string[];
	due?: string[];
	at?: string[];
	namedTimes?: Record<string, number>;
	rules?: Partial<Record<RuleKind, string[]>>;
	priorities?: Record<string, string[]>;
	pin?: string[];
};

const WEEKDAYS = new Map<string, number>();
const TODAY = new Set<string>();
const TOMORROW = new Set<string>();
const NEXT_WEEK = new Set<string>();
const NEXT_MONTH = new Set<string>();
const NEXT = new Set<string>();
const IN = new Set<string>();
const EVERY = new Set<string>();
const END = new Set<string>();
const DUE = new Set<string>();
const AT = new Set<string>();
const UNITS = new Map<string, 'd' | 'w' | 'm'>();
const NAMED_TIMES = new Map<string, number>();
const RULE_WORDS = new Map<string, RuleKind>();
const PRIORITIES = new Map<string, number>();
const PIN = new Set<string>();

for (const lang of Object.values(langs as unknown as Record<string, LangVocab>)) {
	lang.weekdays?.forEach((names, i) => names.forEach((n) => WEEKDAYS.set(n, i + 1)));
	lang.today?.forEach((w) => TODAY.add(w));
	lang.tomorrow?.forEach((w) => TOMORROW.add(w));
	lang.nextWeek?.forEach((w) => NEXT_WEEK.add(w));
	lang.nextMonth?.forEach((w) => NEXT_MONTH.add(w));
	lang.next?.forEach((w) => NEXT.add(w));
	lang.in?.forEach((w) => IN.add(w));
	lang.every?.forEach((w) => EVERY.add(w));
	lang.end?.forEach((w) => END.add(w));
	lang.due?.forEach((w) => DUE.add(w));
	lang.at?.forEach((w) => AT.add(w));
	lang.day?.forEach((w) => UNITS.set(w, 'd'));
	lang.week?.forEach((w) => UNITS.set(w, 'w'));
	lang.month?.forEach((w) => UNITS.set(w, 'm'));
	for (const [w, mins] of Object.entries(lang.namedTimes ?? {})) NAMED_TIMES.set(w, mins);
	for (const [kind, words] of Object.entries(lang.rules ?? {}))
		for (const w of words ?? []) RULE_WORDS.set(w, kind as RuleKind);
	for (const [p, words] of Object.entries(lang.priorities ?? {}))
		for (const w of words ?? []) PRIORITIES.set(w, Number(p));
	lang.pin?.forEach((w) => PIN.add(w));
}

// language-neutral forms: unit letters and p0..p4 / digit priorities
UNITS.set('d', 'd').set('w', 'w').set('m', 'm');
for (let p = 0; p <= 4; p++) {
	PRIORITIES.set(`p${p}`, p);
	PRIORITIES.set(String(p), p);
}

function addDays(date: Date, days: number): Date {
	const next = new Date(date);
	next.setDate(next.getDate() + days);
	return next;
}

function addMonths(date: Date, months: number): Date {
	const next = new Date(date);
	next.setMonth(next.getMonth() + months);
	return next;
}

function isoWeekday(date: Date): number {
	return date.getDay() === 0 ? 7 : date.getDay();
}

// first occurrence of the iso weekday strictly after today
function nextWeekday(today: Date, weekday: number): Date {
	const delta = (((weekday - isoWeekday(today)) % 7) + 7) % 7 || 7;
	return addDays(today, delta);
}

function parseDateToken(token: string, today: Date): Date | null {
	if (TODAY.has(token)) return today;
	if (TOMORROW.has(token)) return addDays(today, 1);
	if (NEXT_WEEK.has(token)) return addDays(today, 7);
	if (NEXT_MONTH.has(token)) return addMonths(today, 1);
	if (/^\d{4}-\d{2}-\d{2}$/.test(token)) {
		const date = new Date(`${token}T00:00:00`);
		return Number.isNaN(date.getTime()) ? null : date;
	}
	const weekday = WEEKDAYS.get(token);
	if (weekday) return nextWeekday(today, weekday);
	return null;
}

// "15:00", "9:30", "3pm", "3:45pm", "18h30" -> minutes since midnight
function parseTimeToken(token: string): number | null {
	const clock = /^(\d{1,2}):(\d{2})$/.exec(token);
	if (clock) {
		const h = Number(clock[1]);
		const m = Number(clock[2]);
		if (h > 23 || m > 59) return null;
		return h * 60 + m;
	}
	const ampm = /^(\d{1,2})(?::(\d{2}))?(am|pm)$/.exec(token);
	if (ampm) {
		let h = Number(ampm[1]);
		const m = Number(ampm[2] ?? 0);
		if (h < 1 || h > 12 || m > 59) return null;
		if (ampm[3] === 'pm' && h !== 12) h += 12;
		if (ampm[3] === 'am' && h === 12) h = 0;
		return h * 60 + m;
	}
	const hour = /^(\d{1,2})h(\d{2})?$/.exec(token);
	if (hour) {
		const h = Number(hour[1]);
		const m = Number(hour[2] ?? 0);
		if (h > 23 || m > 59) return null;
		return h * 60 + m;
	}
	return null;
}

// "~90", "~90m", "~2h", "~1h30m", "~1.5h" -> minutes, same format the cli's
// --estimate flag accepts
function parseEstimateToken(token: string): number | null {
	const inner = /^~(.+)$/.exec(token)?.[1];
	if (!inner) return null;
	const hours = /^(\d+(?:\.\d+)?)h(?:(\d+)m?)?$/.exec(inner);
	if (hours) return Math.round(Number(hours[1]) * 60) + Number(hours[2] ?? 0);
	const mins = /^(\d+)m?$/.exec(inner);
	if (mins) return Number(mins[1]);
	return null;
}

// "<in> 3d" / "<in> 3 days" -> [due date, tokens consumed]
function parseInSpan(
	next: string,
	afterNext: string | undefined,
	today: Date
): [Date, number] | null {
	const span = /^(\d+)(\p{L}+)?$/u.exec(next);
	if (!span) return null;
	const n = Number(span[1]);
	const glued = span[2] !== undefined;
	const unit = UNITS.get(glued ? span[2] : (afterNext ?? ''));
	if (!unit) return null;
	const due =
		unit === 'd' ? addDays(today, n) : unit === 'w' ? addDays(today, n * 7) : addMonths(today, n);
	return [due, glued ? 2 : 3];
}

// bare "daily" / "weekly" / "monthly" only count as a rule as the last word,
// so titles like "weekly sync" survive; the "weekly:N" / "monthly:N" storage
// form is unambiguous and works anywhere
function normalizeRule(token: string, today: Date, isLast: boolean): string | null {
	const bare = RULE_WORDS.get(token);
	if (bare) {
		if (!isLast) return null;
		if (bare === 'daily') return 'daily';
		if (bare === 'weekly') return `weekly:${isoWeekday(today)}`;
		return `monthly:${today.getDate()}`;
	}
	const [kind, arg] = token.split(':');
	if (kind === 'weekly') {
		const weekday = WEEKDAYS.get(arg) ?? (/^\d$/.test(arg) ? Number(arg) : 0);
		if (weekday >= 1 && weekday <= 7) return `weekly:${weekday}`;
	}
	if (kind === 'monthly' && /^\d{1,2}$/.test(arg)) {
		const day = Number(arg);
		if (day >= 1 && day <= 31) return `monthly:${day}`;
	}
	return null;
}

export function parseQuickAdd(input: string, now: Date = new Date()): ParsedQuickAdd {
	const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	const tokens = input.split(/\s+/).filter(Boolean);
	const title: string[] = [];
	const result: ParsedQuickAdd = {
		title: '',
		projectKey: null,
		labelNames: [],
		priority: null,
		pinned: false,
		dueDate: null,
		endDate: null,
		reminderAt: null,
		recurrenceRule: null,
		estimateMinutes: null
	};

	let due: Date | null = null;
	let dueExplicit = false;
	// reminder time-of-day in minutes; converted to an instant after the scan
	let timeOfDay: number | null = null;
	let i = 0;

	while (i < tokens.length) {
		const raw = tokens[i];
		const t = raw.toLowerCase();
		const next = tokens[i + 1]?.toLowerCase();

		// lead words consume the following token only when it matches, so a
		// plain "meet at home" keeps both words in the title
		if (next !== undefined) {
			if (NEXT.has(t)) {
				const weekday = WEEKDAYS.get(next);
				if (weekday) {
					due = addDays(nextWeekday(today, weekday), 7);
					dueExplicit = true;
					i += 2;
					continue;
				}
				const unit = UNITS.get(next);
				if (unit === 'w') {
					due = addDays(today, 7);
					dueExplicit = true;
					i += 2;
					continue;
				}
				if (unit === 'm') {
					due = addMonths(today, 1);
					dueExplicit = true;
					i += 2;
					continue;
				}
			}
			if (IN.has(t)) {
				const span = parseInSpan(next, tokens[i + 2]?.toLowerCase(), today);
				if (span) {
					due = span[0];
					dueExplicit = true;
					i += span[1];
					continue;
				}
			}
			if (EVERY.has(t)) {
				const weekday = WEEKDAYS.get(next);
				const unit = UNITS.get(next);
				if (unit === 'd') {
					result.recurrenceRule = 'daily';
					i += 2;
					continue;
				}
				if (unit === 'w') {
					result.recurrenceRule = `weekly:${isoWeekday(today)}`;
					i += 2;
					continue;
				}
				if (unit === 'm') {
					result.recurrenceRule = `monthly:${today.getDate()}`;
					i += 2;
					continue;
				}
				if (weekday) {
					result.recurrenceRule = `weekly:${weekday}`;
					i += 2;
					continue;
				}
				if (/^\d{1,2}$/.test(next) && Number(next) >= 1 && Number(next) <= 31) {
					result.recurrenceRule = `monthly:${Number(next)}`;
					i += 2;
					continue;
				}
			}
			if (END.has(t)) {
				const end = parseDateToken(next, today);
				if (end) {
					result.endDate = isoDate(end);
					i += 2;
					continue;
				}
			}
			if (DUE.has(t)) {
				const date = parseDateToken(next, today);
				if (date) {
					due = date;
					dueExplicit = true;
					i += 2;
					continue;
				}
			}
			if (AT.has(t)) {
				// "<at> 18" accepts a bare hour on top of the regular time forms
				const time =
					parseTimeToken(next) ??
					(/^\d{1,2}$/.test(next) && Number(next) <= 23 ? Number(next) * 60 : null);
				if (time !== null) {
					timeOfDay = time;
					i += 2;
					continue;
				}
			}
		}

		const date = parseDateToken(t, today);
		if (date) {
			due = date;
			dueExplicit = true;
			i += 1;
			continue;
		}

		const rule = normalizeRule(t, today, i === tokens.length - 1);
		if (rule) {
			result.recurrenceRule = rule;
			i += 1;
			continue;
		}

		const time = parseTimeToken(t);
		if (time !== null) {
			timeOfDay = time;
			i += 1;
			continue;
		}
		const namedTime = dueExplicit ? NAMED_TIMES.get(t) : undefined;
		if (namedTime !== undefined) {
			timeOfDay = namedTime;
			i += 1;
			continue;
		}

		if (t.startsWith('#') && t.length > 1) {
			result.projectKey = t.slice(1);
			i += 1;
			continue;
		}
		if (t.startsWith('+') && t.length > 1) {
			result.labelNames.push(t.slice(1));
			i += 1;
			continue;
		}
		if (t.startsWith('!') && t.length > 1) {
			const flag = t.slice(1);
			if (PIN.has(flag)) {
				result.pinned = true;
				i += 1;
				continue;
			}
			const priority = PRIORITIES.get(flag);
			if (priority !== undefined) {
				result.priority = priority;
				i += 1;
				continue;
			}
		}
		const estimate = parseEstimateToken(t);
		if (estimate !== null) {
			result.estimateMinutes = estimate;
			i += 1;
			continue;
		}

		title.push(raw);
		i += 1;
	}

	result.title = title.join(' ');
	if (due) result.dueDate = isoDate(due);

	if (timeOfDay !== null) {
		const day = due ?? today;
		let reminder = new Date(
			day.getFullYear(),
			day.getMonth(),
			day.getDate(),
			Math.floor(timeOfDay / 60),
			timeOfDay % 60
		);
		if (!dueExplicit && reminder.getTime() <= now.getTime()) reminder = addDays(reminder, 1);
		result.reminderAt = reminder.toISOString();
	}

	return result;
}
