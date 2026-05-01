import { platform as tauriPlatform } from '@tauri-apps/plugin-os';

export type Platform = 'macos' | 'windows' | 'linux';

let cached: Platform | null = null;

export function platform(): Platform {
	if (cached) return cached;
	const p = tauriPlatform();
	cached = p === 'macos' || p === 'windows' || p === 'linux' ? p : 'linux';
	return cached;
}

export const isMac = (): boolean => platform() === 'macos';
export const isWindows = (): boolean => platform() === 'windows';
export const isLinux = (): boolean => platform() === 'linux';

/** Modifier key label for shortcuts UI ("Cmd" on macOS, "Ctrl" elsewhere). */
export const mod = (): string => (isMac() ? 'Cmd' : 'Ctrl');

/** True if a keyboard event matches the platform's primary modifier. */
export function modKey(e: KeyboardEvent | MouseEvent): boolean {
	return isMac() ? e.metaKey : e.ctrlKey;
}

export type InstallType = 'macos' | 'windows' | 'app-image' | 'linux-package' | 'dev';

let cachedInstallType: InstallType | null = null;

/** Detects how the running app was installed. AppImage / DMG / NSIS support
 * self-update; deb/rpm packages do not (managed by system package manager). */
export async function installType(): Promise<InstallType> {
	if (cachedInstallType) return cachedInstallType;
	const { invoke } = await import('@tauri-apps/api/core');
	cachedInstallType = await invoke<InstallType>('get_install_type');
	return cachedInstallType;
}

/** Whether the in-app updater is supported for this install. */
export async function supportsSelfUpdate(): Promise<boolean> {
	const { invoke } = await import('@tauri-apps/api/core');
	return await invoke<boolean>('supports_self_update');
}
