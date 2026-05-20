import { getCurrentWindow } from '@tauri-apps/api/window';

const FULLSCREEN_EXIT_POLL_MS = 50;
const FULLSCREEN_EXIT_MAX_TICKS = 30;

/**
 * Exit fullscreen if active, then resolve once the macOS fullscreen-exit
 * animation has actually completed. Required before hiding/minimizing the
 * window in fullscreen — calling those mid-fullscreen leaves a blank
 * fullscreen Space.
 */
export async function ensureNotFullscreen(): Promise<void> {
  const win = getCurrentWindow();
  if (!(await win.isFullscreen())) return;
  await win.setFullscreen(false);
  for (let i = 0; i < FULLSCREEN_EXIT_MAX_TICKS; i++) {
    await new Promise((r) => setTimeout(r, FULLSCREEN_EXIT_POLL_MS));
    if (!(await win.isFullscreen())) return;
  }
}
