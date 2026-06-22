import type { AppConfig } from './types';
import fr from '../locales/fr.json';
import en from '../locales/en.json';

export const DEFAULT_LOCALE = 'fr-FR';
export const DEFAULT_TIMEZONE = 'Europe/Paris';

export type MessageKey = keyof typeof fr;

const CATALOGS: Record<string, Record<MessageKey, string>> = { fr, en };
const FALLBACK_LANG = 'en';

export function resolveLocale(config?: AppConfig | null): string {
	return config?.locale ?? DEFAULT_LOCALE;
}

export function resolveTimezone(config?: AppConfig | null): string {
	return config?.timezone ?? DEFAULT_TIMEZONE;
}

/** Locale pour les pages sans config chargée (ex. login). */
export function browserLocale(): string {
	if (typeof navigator !== 'undefined' && navigator.language) {
		return navigator.language;
	}
	return DEFAULT_LOCALE;
}

export function langCode(locale: string): string {
	return locale.split('-')[0]?.toLowerCase() || FALLBACK_LANG;
}

export function t(key: MessageKey, locale: string): string {
	const lang = langCode(locale);
	return CATALOGS[lang]?.[key] ?? CATALOGS[FALLBACK_LANG][key] ?? key;
}

export function formatBytes(bytes: number, locale: string): string {
	if (bytes === 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	const value = bytes / Math.pow(k, i);
	return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value)} ${sizes[i]}`;
}

export function formatUptime(secs: number, locale: string): string {
	const days = Math.floor(secs / 86400);
	const hours = Math.floor((secs % 86400) / 3600);
	const mins = Math.floor((secs % 3600) / 60);
	const fr = langCode(locale) === 'fr';
	if (days > 0) return fr ? `${days}j ${hours}h` : `${days}d ${hours}h`;
	if (hours > 0) return fr ? `${hours}h ${mins}m` : `${hours}h ${mins}m`;
	return `${mins}m`;
}

export function formatForecastDate(isoDate: string, locale: string): string {
	const date = parseIsoDate(isoDate);
	return new Intl.DateTimeFormat(locale, { day: '2-digit', month: '2-digit' }).format(date);
}

export function formatRssDate(value: string, locale: string, timezone: string): string {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return value;
	return new Intl.DateTimeFormat(locale, {
		dateStyle: 'medium',
		timeStyle: 'short',
		timeZone: timezone
	}).format(date);
}

export function formatDateTime(
	date: Date,
	locale: string,
	timezone: string,
	options?: Intl.DateTimeFormatOptions
): string {
	return new Intl.DateTimeFormat(locale, {
		timeZone: timezone,
		...options
	}).format(date);
}

export function formatMonthTitle(date: Date, locale: string, timezone: string): string {
	return new Intl.DateTimeFormat(locale, {
		month: 'long',
		year: 'numeric',
		timeZone: timezone
	}).format(date);
}

export function weekdayLabels(locale: string, firstDay: number): string[] {
	const base = new Date(Date.UTC(2024, 0, 1)); // Monday
	const labels: string[] = [];
	for (let i = 0; i < 7; i++) {
		const day = new Date(base);
		day.setUTCDate(base.getUTCDate() + i);
		labels.push(
			new Intl.DateTimeFormat(locale, { weekday: 'narrow' }).format(day)
		);
	}
	if (firstDay === 0) {
		const sunday = labels.pop();
		if (sunday) labels.unshift(sunday);
	}
	return labels;
}

export function firstDayOfWeek(locale: string): number {
	try {
		const intlLocale = new Intl.Locale(locale) as Intl.Locale & {
			weekInfo?: { firstDay?: number };
		};
		const firstDay = intlLocale.weekInfo?.firstDay;
		if (firstDay != null) return firstDay === 7 ? 0 : firstDay;
	} catch {
		// Intl.Locale.weekInfo not supported
	}
	return langCode(locale) === 'en' ? 0 : 1;
}

export interface CalendarCell {
	day: number;
	inMonth: boolean;
	isToday: boolean;
}

export function buildMonthGrid(
	viewDate: Date,
	locale: string,
	timezone: string,
	today?: { year: number; month: number; day: number }
): { year: number; month: number; cells: CalendarCell[] } {
	const parts = zonedParts(viewDate, timezone);
	const year = parts.year;
	const month = parts.month;

	const daysInMonth = new Date(Date.UTC(year, month, 0)).getUTCDate();
	const firstWeekday = weekdayIndex(Date.UTC(year, month - 1, 1), timezone);
	const startOffset = (firstWeekday - firstDayOfWeek(locale) + 7) % 7;

	const prevMonth = month === 1 ? 12 : month - 1;
	const prevYear = month === 1 ? year - 1 : year;
	const daysInPrevMonth = new Date(Date.UTC(prevYear, prevMonth, 0)).getUTCDate();

	const cells: CalendarCell[] = [];
	for (let i = 0; i < startOffset; i++) {
		const day = daysInPrevMonth - startOffset + i + 1;
		cells.push({
			day,
			inMonth: false,
			isToday: isSameDay(prevYear, prevMonth, day, today)
		});
	}
	for (let day = 1; day <= daysInMonth; day++) {
		cells.push({
			day,
			inMonth: true,
			isToday: isSameDay(year, month, day, today)
		});
	}

	const nextMonth = month === 12 ? 1 : month + 1;
	const nextYear = month === 12 ? year + 1 : year;
	let nextDay = 1;
	while (cells.length % 7 !== 0) {
		cells.push({
			day: nextDay,
			inMonth: false,
			isToday: isSameDay(nextYear, nextMonth, nextDay, today)
		});
		nextDay++;
	}

	return { year, month, cells };
}

/** Décale viewDate d'un nombre de mois dans le fuseau configuré. */
export function shiftMonth(viewDate: Date, delta: number, timezone: string): Date {
	const parts = zonedParts(viewDate, timezone);
	let month = parts.month + delta;
	let year = parts.year;
	while (month > 12) {
		month -= 12;
		year++;
	}
	while (month < 1) {
		month += 12;
		year--;
	}
	const maxDay = new Date(Date.UTC(year, month, 0)).getUTCDate();
	const day = Math.min(parts.day, maxDay);
	return new Date(Date.UTC(year, month - 1, day, 12));
}

function isSameDay(
	year: number,
	month: number,
	day: number,
	today?: { year: number; month: number; day: number }
): boolean {
	if (!today) return false;
	return year === today.year && month === today.month && day === today.day;
}

function parseIsoDate(isoDate: string): Date {
	const [y, m, d] = isoDate.split('-').map(Number);
	return new Date(y, (m ?? 1) - 1, d ?? 1);
}

export function zonedParts(date: Date, timezone: string) {
	const parts = new Intl.DateTimeFormat('en-US', {
		timeZone: timezone,
		year: 'numeric',
		month: 'numeric',
		day: 'numeric'
	}).formatToParts(date);

	const get = (type: Intl.DateTimeFormatPartTypes) =>
		Number(parts.find((p) => p.type === type)?.value ?? 0);

	return { year: get('year'), month: get('month'), day: get('day') };
}

function weekdayIndex(utcMs: number, timezone: string): number {
	const parts = new Intl.DateTimeFormat('en-US', {
		timeZone: timezone,
		weekday: 'short'
	}).formatToParts(new Date(utcMs));
	const wd = parts.find((p) => p.type === 'weekday')?.value ?? 'Mon';
	const map: Record<string, number> = {
		Sun: 0,
		Mon: 1,
		Tue: 2,
		Wed: 3,
		Thu: 4,
		Fri: 5,
		Sat: 6
	};
	return map[wd] ?? 1;
}
