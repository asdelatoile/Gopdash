import type { WidgetType } from './types';

export type ParsedIcon =
	| { kind: 'lucide'; name: string }
	| { kind: 'image'; src: string };

const SELFH_CDN = 'https://cdn.jsdelivr.net/gh/selfhst/icons';
const LUCIDE_CDN = 'https://cdn.jsdelivr.net/npm/lucide-static@0.511.0/icons';

const DEFAULT_WIDGET_ICONS: Record<WidgetType, string> = {
	system: 'lucide:cpu',
	weather: 'lucide:cloud-sun',
	docker: 'lucide:container',
	bookmarks: 'lucide:bookmark',
	rss: 'lucide:rss',
	calendar: 'lucide:calendar',
	search: 'lucide:search'
};

export function defaultWidgetIcon(type: WidgetType): string {
	return DEFAULT_WIDGET_ICONS[type];
}

export function lucideIconUrl(name: string): string {
	return `${LUCIDE_CDN}/${name}.svg`;
}

export function parseIcon(value?: string | null): ParsedIcon | null {
	if (!value?.trim()) return null;

	const trimmed = value.trim();

	if (trimmed.startsWith('lucide:')) {
		return { kind: 'lucide', name: normalizeLucideName(trimmed.slice(7)) };
	}
	if (trimmed.startsWith('lucide-')) {
		return { kind: 'lucide', name: normalizeLucideName(trimmed.slice(7)) };
	}

	if (trimmed.startsWith('sh:') || trimmed.startsWith('selfh:')) {
		return { kind: 'image', src: selfhUrl(trimmed.split(':').slice(1).join(':')) };
	}
	if (trimmed.startsWith('sh-')) {
		return { kind: 'image', src: selfhUrl(trimmed.slice(3)) };
	}

	if (trimmed.startsWith('/icons/')) {
		return { kind: 'image', src: trimmed };
	}
	if (trimmed.startsWith('icons/')) {
		return { kind: 'image', src: `/${trimmed}` };
	}

	if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
		return { kind: 'image', src: trimmed };
	}

	if (/\.(png|svg|webp|avif)$/i.test(trimmed) && !trimmed.includes('/')) {
		return { kind: 'image', src: selfhUrl(trimmed) };
	}

	if (!trimmed.includes('/') && !trimmed.includes(':')) {
		return { kind: 'image', src: selfhUrl(`${trimmed}.webp`) };
	}

	return { kind: 'lucide', name: normalizeLucideName(trimmed) };
}

function normalizeLucideName(name: string): string {
	return name
		.trim()
		.replace(/_/g, '-')
		.replace(/([a-z0-9])([A-Z])/g, '$1-$2')
		.toLowerCase();
}

function selfhUrl(reference: string): string {
	const ref = reference.trim().replace(/\s+/g, '-').toLowerCase();
	const extMatch = ref.match(/\.(png|svg|webp|avif)$/i);
	if (extMatch) {
		const ext = extMatch[1].toLowerCase();
		const name = ref.replace(/\.(png|svg|webp|avif)$/i, '');
		return `${SELFH_CDN}/${ext}/${name}.${ext}`;
	}
	return `${SELFH_CDN}/webp/${ref}.webp`;
}
