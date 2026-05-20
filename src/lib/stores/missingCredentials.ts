// Post-restore notice. Cloud sync deliberately doesn't replicate
// passwords / key files to a fresh device (those are per-machine), so
// right after a successful `cloud_sync_restore` we probe each mode for
// any rows that came back credential-less and surface a single transient
// banner naming the affected modes. The probe + the banner stop here —
// we trust developers to fix it from the connection's edit dialog once
// they know which modes to look at. Auto-hides after a few seconds so
// it doesn't linger.

import { writable } from 'svelte/store';
import { cloudProbeMissingCredentials } from '$lib/commands/cloud';

export type RestoreMode = 'ssh' | 'sql' | 'nosql' | 'explorer';

export interface RestoreNoticeState {
  /** Modes that have at least one credential-missing row. Empty = hide. */
  modes: RestoreMode[];
}

export const restoreNotice = writable<RestoreNoticeState>({ modes: [] });

/** Auto-hide timer; one global instance so consecutive restores reset
 *  cleanly instead of stacking. */
let hideTimer: ReturnType<typeof setTimeout> | null = null;

/** How long the banner stays up. Long enough for a developer to read
 *  + decide which mode to open; short enough not to pollute the UI. */
const VISIBLE_MS = 12_000;

/** Run the probe and show the banner if any mode needs attention.
 *  Call this right after `cloudSyncRestore()` completes. Returns `true`
 *  when the banner was shown so the caller can suppress its redundant
 *  "Restored from cloud" toast; returns `false` for clean restores so
 *  the caller still confirms the action with a toast. Silent on probe
 *  failure — the banner is a nice-to-have, not load-bearing. */
export async function announceRestoreCompletion(): Promise<boolean> {
  try {
    const m = await cloudProbeMissingCredentials();
    const modes: RestoreMode[] = [];
    if (m.ssh.length) modes.push('ssh');
    if (m.sql.length) modes.push('sql');
    if (m.nosql.length) modes.push('nosql');
    if (m.explorer.length) modes.push('explorer');
    if (modes.length === 0) return false;

    restoreNotice.set({ modes });
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      restoreNotice.set({ modes: [] });
      hideTimer = null;
    }, VISIBLE_MS);
    return true;
  } catch (e) {
    console.error('post-restore probe:', e);
    return false;
  }
}

export function dismissRestoreNotice() {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  restoreNotice.set({ modes: [] });
}
