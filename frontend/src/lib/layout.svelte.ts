const STORAGE_KEY = 'gopdash-layout-locked';

export const layoutState = $state({
	locked: true,
	savedNotice: false
});

let savedNoticeTimer: ReturnType<typeof setTimeout> | null = null;

export function notifyLayoutSaved() {
	if (savedNoticeTimer) clearTimeout(savedNoticeTimer);
	layoutState.savedNotice = true;
	savedNoticeTimer = setTimeout(() => {
		layoutState.savedNotice = false;
		savedNoticeTimer = null;
	}, 2000);
}

export function initLayoutLock() {
	if (typeof localStorage === 'undefined') return;
	layoutState.locked = localStorage.getItem(STORAGE_KEY) !== 'false';
}

export function toggleLayoutLock() {
	layoutState.locked = !layoutState.locked;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(STORAGE_KEY, String(layoutState.locked));
	}
}

export function setLayoutLocked(locked: boolean) {
	layoutState.locked = locked;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(STORAGE_KEY, String(locked));
	}
}
