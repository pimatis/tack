import { isoDate } from '$lib/task/recurrence';

// natural language quick-add parser, mirrored by the cli port in
// src-tauri/cli/src/quickadd.rs and kept in sync through the shared fixture in
// ./cases.json. pure function: no db access, project/label resolution happens
// in the caller. recognized tokens:
//   #key                       project (prefix or name, case-insensitive)
//   +name                      label (existing label name)
//   !urgent !high !medium !low !none !p0..p4   priority
//   !pin !pinned               pin the task
//   ~90m ~2h ~1h30m ~90        estimate in minutes
//   today tomorrow YYYY-MM-DD mon..sun   due date; weekdays resolve to the
//                              next occurrence strictly after today;
//                              turkish aliases: bugün yarın pazartesi..pazar
//   next <weekday|week|month>  due date pushed one step further (gelecek)
//   in <N>d|w|m                due date N days/weeks/months out
//   on|by|due <date>           due date
//   end|until <date>           end date
//   HH:MM, 3pm, 3:45pm, at <time>        reminder; lands on the due date when
//                              one was given, else today (or tomorrow once the
//                              time has already passed)
//   daily weekly monthly weekly:N monthly:N   recurrence rule (storage format)
//   every day|week|month|<weekday>|<1-31>     same, natural form (her, with
//                              gün/hafta/ay for the turkish forms)
// anything unrecognized stays in the title
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

const WEEKDAYS: Record<string, number> = {
	mon: 1,
	monday: 1,
	tue: 2,
	tues: 2,
	tuesday: 2,
	wed: 3,
	wednesday: 3,
	thu: 4,
	thur: 4,
	thurs: 4,
	thursday: 4,
	fri: 5,
	friday: 5,
	sat: 6,
	saturday: 6,
	sun: 7,
	sunday: 7,
	// turkish weekdays; pzt disambiguates pazartesi from paz
	pazartesi: 1,
	pzt: 1,
	salı: 2,
	sal: 2,
	çarşamba: 3,
	çar: 3,
	perşembe: 4,
	per: 4,
	cuma: 5,
	cum: 5,
	cumartesi: 6,
	cmt: 6,
	pazar: 7,
	paz: 7
};

const PRIORITIES: Record<string, number> = {
	none: 0,
	no: 0,
	p0: 0,
	'0': 0,
	urgent: 1,
	p1: 1,
	'1': 1,
	high: 2,
	p2: 2,
	'2': 2,
	medium: 3,
	med: 3,
	p3: 3,
	'3': 3,
	low: 4,
	p4: 4,
	'4': 4
};

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

// named times of day; consumed only after an explicit date token so plain
// titles like "akşam yemeği" survive
const NAMED_TIMES: Record<string, number> = {
	sabah: 9 * 60,
	öğle: 12 * 60,
	öğlen: 12 * 60,
	akşam: 19 * 60,
	gece: 22 * 60
};

function parseDateToken(token: string, today: Date): Date | null {
	if (token === 'today' || token === 'bugün') return today;
	if (token === 'tomorrow' || token === 'yarın') return addDays(today, 1);
	if (token === 'haftaya') return addDays(today, 7);
	if (/^\d{4}-\d{2}-\d{2}$/.test(token)) {
		const date = new Date(`${token}T00:00:00`);
		return Number.isNaN(date.getTime()) ? null : date;
	}
	const weekday = WEEKDAYS[token];
	if (weekday) return nextWeekday(today, weekday);
	return null;
}

// "15:00", "9:30", "3pm", "3:45pm" -> minutes since midnight
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

// bare "daily" / "weekly" / "monthly" only count as a rule as the last word,
// so titles like "weekly sync" survive; the "weekly:N" / "monthly:N" storage
// form is unambiguous and works anywhere
function normalizeRule(token: string, today: Date, isLast: boolean): string | null {
	if (token === 'daily') return isLast ? 'daily' : null;
	if (token === 'weekly') return isLast ? `weekly:${isoWeekday(today)}` : null;
	if (token === 'monthly') return isLast ? `monthly:${today.getDate()}` : null;
	const [kind, arg] = token.split(':');
	if (kind === 'weekly') {
		const weekday = WEEKDAYS[arg] ?? (/^\d$/.test(arg) ? Number(arg) : 0);
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
			if (t === 'next' || t === 'gelecek' || t === 'haftaya') {
				const weekday = WEEKDAYS[next];
				if (weekday) {
					due = addDays(nextWeekday(today, weekday), 7);
					dueExplicit = true;
					i += 2;
					continue;
				}
				if (next === 'week') {
					due = addDays(today, 7);
					dueExplicit = true;
					i += 2;
					continue;
				}
				if (next === 'month') {
					due = addMonths(today, 1);
					dueExplicit = true;
					i += 2;
					continue;
				}
			}
			if (t === 'in') {
				const span = /^(\d+)([dwm])$/.exec(next);
				if (span) {
					const n = Number(span[1]);
					due =
						span[2] === 'd'
							? addDays(today, n)
							: span[2] === 'w'
								? addDays(today, n * 7)
								: addMonths(today, n);
					dueExplicit = true;
					i += 2;
					continue;
				}
			}
			if (t === 'every' || t === 'her') {
				const weekday = WEEKDAYS[next];
				if (next === 'day' || next === 'gün') {
					result.recurrenceRule = 'daily';
					i += 2;
					continue;
				}
				if (next === 'week' || next === 'hafta') {
					result.recurrenceRule = `weekly:${isoWeekday(today)}`;
					i += 2;
					continue;
				}
				if (next === 'month' || next === 'ay') {
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
			if (t === 'end' || t === 'until') {
				const end = parseDateToken(next, today);
				if (end) {
					result.endDate = isoDate(end);
					i += 2;
					continue;
				}
			}
			if (t === 'on' || t === 'by' || t === 'due') {
				const date = parseDateToken(next, today);
				if (date) {
					due = date;
					dueExplicit = true;
					i += 2;
					continue;
				}
			}
			if (t === 'at') {
				const time = parseTimeToken(next);
				if (time !== null) {
					timeOfDay = time;
					i += 2;
					continue;
				}
			}
			if (t === 'saat') {
				// "saat 18" accepts a bare hour on top of the regular time forms
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
		if (dueExplicit && t in NAMED_TIMES) {
			timeOfDay = NAMED_TIMES[t];
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
			if (flag === 'pin' || flag === 'pinned') {
				result.pinned = true;
				i += 1;
				continue;
			}
			if (flag in PRIORITIES) {
				result.priority = PRIORITIES[flag];
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
