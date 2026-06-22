export type WidgetType = 'docker' | 'system' | 'weather' | 'bookmarks' | 'rss' | 'calendar' | 'search';
export type DockerGroupBy = 'flat' | 'compose';
export type SearchTarget = 'new-tab' | 'same-tab';

interface BaseWidget {
	id: string;
	title?: string;
	icon?: string;
	/** Affiche le header (titre + icône). Défaut : true */
	show_header?: boolean;
	x: number;
	y: number;
	w: number;
	h: number;
}

export type WidgetConfig =
	| (BaseWidget & { type: 'system' })
	| (BaseWidget & {
			type: 'docker';
			containers?: string[];
			show_all?: boolean;
			group_by?: DockerGroupBy;
			collapse_groups?: boolean;
	  })
	| (BaseWidget & { type: 'weather'; location?: string; units?: string; show_forecast?: boolean })
	| (BaseWidget & { type: 'bookmarks'; group?: string })
	| (BaseWidget & { type: 'rss'; feed?: string; max_items?: number })
	| (BaseWidget & {
			type: 'calendar';
			/** Affiche le jour et l'heure actuels au-dessus de la grille (défaut : true) */
			show_today?: boolean;
			/** Affiche les jours des mois adjacentes (défaut : false) */
			show_outside_days?: boolean;
			/** Affiche les flèches pour changer de mois (défaut : false) */
			show_navigation?: boolean;
	  })
	| (BaseWidget & {
			type: 'search';
			/** ID du moteur par défaut (services.yaml → search_engines) */
			engine: string;
			/** new-tab (défaut) | same-tab */
			target?: SearchTarget;
	  });

export type ThemeMode = 'light' | 'dark' | 'system';
export type ThemePreset = 'default' | 'ocean' | 'forest' | 'rose';

export interface ThemeConfig {
	mode: ThemeMode;
	preset: ThemePreset;
}

export interface SessionInfo {
	auth_enabled: boolean;
	authenticated: boolean;
	username: string | null;
}

export interface AppConfig {
	title: string;
	refresh_interval: number;
	widgets: WidgetConfig[];
	auth_enabled: boolean;
	weather_configured: boolean;
	weather_show_forecast: boolean;
	bookmark_groups: string[];
	rss_feeds: string[];
	search_engines: SearchEngineConfig[];
	theme: ThemeConfig;
	persist_layout: boolean;
	locale: string;
	timezone: string;
}

export interface LayoutSaveResponse {
	status: string;
	persisted: boolean;
	message?: string;
}

export interface ContainerInfo {
	id: string;
	name: string;
	image: string;
	status: string;
	state: string;
	health: string | null;
	cpu_percent: number;
	memory_usage: number;
	memory_limit: number;
	memory_percent: number;
	is_arr: boolean;
	arr_kind: string | null;
	arr_url: string | null;
	compose_project: string | null;
	compose_service: string | null;
}

export interface SystemMetrics {
	cpu_usage: number;
	cpu_cores: number;
	memory_total: number;
	memory_used: number;
	memory_percent: number;
	disks: DiskInfo[];
	temperatures: TempInfo[];
	hostname: string;
	uptime_secs: number;
}

export interface DiskInfo {
	name: string;
	mount_point: string;
	total: number;
	used: number;
	available: number;
	percent: number;
}

export interface TempInfo {
	label: string;
	celsius: number;
}

export interface WeatherData {
	location: string;
	temp: number;
	feels_like: number;
	humidity: number;
	description: string;
	icon: string;
	wind_speed: number;
	forecast: ForecastDay[];
}

export interface ForecastDay {
	date: string;
	temp_min: number;
	temp_max: number;
	description: string;
	icon: string;
}

export interface BookmarkGroup {
	name: string;
	links: BookmarkLink[];
}

export interface BookmarkLink {
	name: string;
	url: string;
	icon?: string;
	description?: string;
}

export interface RssFeedData {
	name: string;
	items: RssItem[];
}

export interface RssItem {
	title: string;
	link: string;
	pub_date: string | null;
	description: string | null;
}

export interface SearchEngineConfig {
	id: string;
	name: string;
	/** URL avec placeholder `{query}` */
	url: string;
	icon?: string;
}

export interface SSEUpdate {
	type: string;
	system: SystemMetrics;
	docker: ContainerInfo[];
	timestamp: string;
}

export interface WidgetLayout {
	id: string;
	x: number;
	y: number;
	w: number;
	h: number;
}

export function gridItemAttrs(widget: WidgetConfig): Record<string, string | number> {
	return {
		'gs-id': widget.id,
		'gs-x': widget.x,
		'gs-y': widget.y,
		'gs-w': widget.w,
		'gs-h': widget.h
	};
}
