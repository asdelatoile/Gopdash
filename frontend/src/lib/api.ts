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

	getDockerUpdates: (filter?: string, showAll?: boolean) => {
		const params = new URLSearchParams();
		if (filter) params.set('filter', filter);
		if (showAll) params.set('show_all', 'true');
		const qs = params.toString();
		return request<import('./types').ContainerUpdateInfo[]>(`/docker/updates${qs ? `?${qs}` : ''}`);
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

	getBookmarks: (group?: string) => {
		const qs = group ? `?group=${encodeURIComponent(group)}` : '';
		return request<import('./types').BookmarkGroup[]>(`/bookmarks${qs}`);
	},

	getRss: (feed: string) => request<import('./types').RssFeedData>(`/rss/${encodeURIComponent(feed)}`),

	getBookmarkHealth: (group?: string) => {
		const qs = group ? `?group=${encodeURIComponent(group)}` : '';
		return request<import('./types').BookmarkHealthResult[]>(`/bookmarks/health${qs}`);
	},

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
