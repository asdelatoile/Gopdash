import type { SessionInfo } from './types';

export type { SessionInfo };

export const authState = $state({
	checked: false,
	authEnabled: false,
	authenticated: false,
	username: null as string | null
});

export async function checkSession(): Promise<SessionInfo> {
	const { api } = await import('./api');
	try {
		const session = await api.getSession();
		authState.authEnabled = session.auth_enabled;
		authState.authenticated = session.authenticated;
		authState.username = session.username;
		return session;
	} catch {
		// Endpoint inaccessible — traiter comme non authentifié pour débloquer l'UI
		authState.authEnabled = true;
		authState.authenticated = false;
		authState.username = null;
		return {
			auth_enabled: true,
			authenticated: false,
			username: null
		};
	} finally {
		authState.checked = true;
	}
}

export async function login(username: string, password: string): Promise<void> {
	const { api } = await import('./api');
	await api.login(username, password);
	await checkSession();
}

export async function logout(): Promise<void> {
	const { api } = await import('./api');
	await api.logout();
	authState.authenticated = false;
	authState.username = null;
}
