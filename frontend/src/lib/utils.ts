import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// shadcn-svelte utility types
export type WithElementRef<T> = T & { ref?: HTMLElement | null };

export type WithoutChildren<T> = T extends { children?: unknown } ? Omit<T, 'children'> : T;
export type WithoutChild<T> = T extends { child?: unknown } ? Omit<T, 'child'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;

import { formatBytes as formatBytesDefault, formatUptime as formatUptimeDefault } from './locale';

export function formatBytes(bytes: number, locale = 'fr-FR'): string {
	return formatBytesDefault(bytes, locale);
}

export function formatUptime(secs: number, locale = 'fr-FR'): string {
	return formatUptimeDefault(secs, locale);
}
