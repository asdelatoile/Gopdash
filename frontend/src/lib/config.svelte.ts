import type { AppConfig } from './types';
import { api } from './api';
import { applyTheme } from './theme.svelte';

export const appConfig = $state({
	data: null as AppConfig | null,
	loading: true,
	error: null as string | null
});

let fetchPromise: Promise<void> | null = null;
let inFlightLoads = 0;

export async function fetchConfig(options?: { silent?: boolean }) {
	if (fetchPromise) {
		await fetchPromise;
		return;
	}

	const silent = options?.silent ?? false;

	fetchPromise = (async () => {
		if (!silent) {
			inFlightLoads++;
			appConfig.loading = true;
		}
		appConfig.error = null;
		try {
			appConfig.data = await api.getConfig();
			if (appConfig.data?.theme) {
				applyTheme(appConfig.data.theme);
			}
		} catch (e) {
			appConfig.error = e instanceof Error ? e.message : 'Failed to load config';
		} finally {
			if (!silent) {
				inFlightLoads = Math.max(0, inFlightLoads - 1);
				if (inFlightLoads === 0) {
					appConfig.loading = false;
				}
			}
		}
	})();

	try {
		await fetchPromise;
	} finally {
		fetchPromise = null;
	}
}
