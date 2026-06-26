const API_BASE = '/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), 15_000);

	try {
		const res = await fetch(`${API_BASE}${path}`, {
			credentials: 'include',
			...options,
			signal: controller.signal,
			headers: {
				'Content-Type': 'application/json',
				...options?.headers
			}
		});

		if (!res.ok) {
			const body = await res.json().catch(() => ({ error: res.statusText }));
			throw new Error(body.error || `HTTP ${res.status}`);
		}

		return res.json();
	} catch (e) {
		if (e instanceof DOMException && e.name === 'AbortError') {
			throw new Error('Request timed out');
		}
		throw e;
	} finally {
		clearTimeout(timeout);
	}
}

export const api = {
	getSession: () => request<import('./types').SessionInfo>('/auth/session'),

	login: (username: string, password: string) =>
		request<{ status: string; username: string }>('/auth/login', {
			method: 'POST',
			body: JSON.stringify({ username, password })
		}),

	logout: () =>
		request<{ status: string }>('/auth/logout', {
			method: 'POST'
		}),

	getConfig: () => request<import('./types').AppConfig>('/config'),

	getContainers: (filter?: string, showAll?: boolean) => {
		const params = new URLSearchParams();
		if (filter) params.set('filter', filter);
		if (showAll) params.set('show_all', 'true');
		const qs = params.toString();
		return request<import('./types').ContainerInfo[]>(`/docker/containers${qs ? `?${qs}` : ''}`);
	},

	startContainer: (id: string) =>
		request(`/docker/containers/${id}/start`, { method: 'POST' }),

	stopContainer: (id: string) =>
		request(`/docker/containers/${id}/stop`, { method: 'POST' }),

	restartContainer: (id: string) =>
		request(`/docker/containers/${id}/restart`, { method: 'POST' }),

	getDockerUpdates: (filter?: string, showAll?: boolean, force?: boolean) => {
		const params = new URLSearchParams();
		if (filter) params.set('filter', filter);
		if (showAll) params.set('show_all', 'true');
		if (force) params.set('force', 'true');
		const qs = params.toString();
		return request<import('./types').DockerUpdatesResponse>(`/docker/updates${qs ? `?${qs}` : ''}`);
	},

	updateContainer: (id: string) =>
		request(`/docker/containers/${id}/update`, { method: 'POST' }),

	pruneUnusedImages: () =>
		request<import('./types').ImagePruneResult>('/docker/images/prune', { method: 'POST' }),

	getSystem: () => request<import('./types').SystemMetrics>('/system'),

	getWeather: (location?: string) => {
		const qs = location ? `?location=${encodeURIComponent(location)}` : '';
		return request<import('./types').WeatherData>(`/weather${qs}`);
	},

	getBookmarks: (serviceId?: string) => {
		const qs = serviceId ? `?service_id=${encodeURIComponent(serviceId)}` : '';
		return request<import('./types').BookmarkGroup[]>(`/bookmarks${qs}`);
	},

	getRss: (serviceId: string) =>
		request<import('./types').RssFeedData>(`/rss/${encodeURIComponent(serviceId)}`),

	getBookmarkHealth: (serviceId?: string) => {
		const qs = serviceId ? `?service_id=${encodeURIComponent(serviceId)}` : '';
		return request<import('./types').BookmarkHealthResult[]>(`/bookmarks/health${qs}`);
	},

	getJellyfinStatus: (options?: {
		showNowPlaying?: boolean;
		showLibraryCounts?: boolean;
		maxSessions?: number;
	}) => {
		const params = new URLSearchParams();
		if (options?.showNowPlaying === false) params.set('show_now_playing', 'false');
		if (options?.showLibraryCounts === false) params.set('show_library_counts', 'false');
		if (options?.maxSessions != null) params.set('max_sessions', String(options.maxSessions));
		const qs = params.toString();
		return request<import('./types').JellyfinStatus>(`/jellyfin/status${qs ? `?${qs}` : ''}`);
	},

	jellyfinImageUrl: (itemId: string) => `${API_BASE}/jellyfin/images/${encodeURIComponent(itemId)}`,

	getHomeAssistantState: (widgetId: string, force?: boolean) => {
		const params = new URLSearchParams({ widget_id: widgetId });
		if (force) params.set('force', 'true');
		return request<import('./types').HomeAssistantWidgetState>(
			`/homeassistant/state?${params.toString()}`
		);
	},

	setHomeAssistantSwitch: (widgetId: string, entityId: string, on: boolean) =>
		request<import('./types').HaSwitchState>('/homeassistant/switch', {
			method: 'POST',
			body: JSON.stringify({ widget_id: widgetId, entity_id: entityId, on })
		}),

	saveLayout: (widgets: import('./types').WidgetLayout[]) =>
		request<import('./types').LayoutSaveResponse>('/config/layout', {
			method: 'PUT',
			body: JSON.stringify({ widgets })
		})
};

export function createSSE(
	onUpdate: (data: import('./types').SSEUpdate) => void,
	onConfigReload?: () => void,
	onConnectionChange?: (connected: boolean) => void
): () => void {
	let source: EventSource | null = null;
	let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	let closed = false;

	function connect() {
		if (closed) return;

		source?.close();
		source = new EventSource(`${API_BASE}/events`, { withCredentials: true });

		source.onopen = () => {
			onConnectionChange?.(true);
		};

		source.addEventListener('update', (e) => {
			try {
				const data = JSON.parse(e.data) as import('./types').SSEUpdate;
				onUpdate(data);
			} catch (err) {
				console.error('SSE parse error:', err);
			}
		});

		source.addEventListener('config_reload', () => {
			onConfigReload?.();
		});

		source.onerror = () => {
			onConnectionChange?.(false);
			source?.close();
			source = null;

			if (!closed) {
				reconnectTimer = setTimeout(connect, 3000);
			}
		};
	}

	connect();

	return () => {
		closed = true;
		if (reconnectTimer) clearTimeout(reconnectTimer);
		source?.close();
		onConnectionChange?.(false);
	};
}
