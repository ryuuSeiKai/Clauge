import { invoke } from '@tauri-apps/api/core';
import type { CloudUser, CloudProviderLink, Provider } from '$lib/stores/cloud';

export interface CloudEntitlements {
  plan: string;
  credits?: {
    remaining: number;
    allowance: number;
    resets_at: string | null;
  };
  subscription?: {
    status: string;
    cancel_at_period_end: boolean;
    is_lifetime?: boolean;
    current_period_end?: string | null;
    current_period_start?: string | null;
    interval?: 'monthly' | 'yearly' | 'lifetime' | null;
    price_usd?: number | null;
  };
}

export interface CloudStatus {
  connected: boolean;
  activeProvider: Provider | null;
  user: CloudUser | null;
  providers: CloudProviderLink[];
  plan: string;
  lastSynced: Record<string, string>;
  entitlements?: CloudEntitlements;
}

export const cloudGetStatus = () =>
  invoke<CloudStatus>('cloud_get_status');

export interface MissingCredentials {
  ssh: string[];
  sql: string[];
  nosql: string[];
  explorer: string[];
}

export const cloudProbeMissingCredentials = () =>
  invoke<MissingCredentials>('cloud_probe_missing_credentials');

export const cloudGithubLoginUrl = (state: string) =>
  invoke<string>('cloud_github_login_url', { state });

export const cloudGoogleLoginUrl = (state: string) =>
  invoke<string>('cloud_google_login_url', { state });

export const cloudCreateTicket = () =>
  invoke<string>('cloud_create_ticket');

export interface TicketPollResult {
  status: string;
  token?: string;
  userId?: number;
}

export const cloudPollTicket = (ticket: string) =>
  invoke<TicketPollResult>('cloud_poll_ticket', { ticket });

export const cloudExchangeCode = (provider: Provider, code: string) =>
  invoke<CloudStatus>('cloud_exchange_code', { provider, code });

export const cloudLinkProvider = (provider: Provider, code: string) =>
  invoke<CloudStatus>('cloud_link_provider', { provider, code });

export const cloudUnlinkProvider = (provider: Provider) =>
  invoke<CloudStatus>('cloud_unlink_provider', { provider });

export const cloudUpdateProfile = (fields: {
  displayName?: string;
  firstName?: string;
  lastName?: string;
}) => invoke<CloudStatus>('cloud_update_profile', {
  displayName: fields.displayName,
  firstName: fields.firstName,
  lastName: fields.lastName,
});

export const cloudCheckRemoteExists = () =>
  invoke<boolean>('cloud_check_remote_exists');

export const cloudSyncPushNow = () =>
  invoke<string[]>('cloud_sync_push_now');

export const cloudSyncRestore = () =>
  invoke<string[]>('cloud_sync_restore');

export const cloudGetConflicts = () =>
  invoke<string[]>('cloud_get_conflicts');

export const cloudResolveKeepLocal = () =>
  invoke<void>('cloud_resolve_keep_local');

export const cloudResolveUseRemote = () =>
  invoke<void>('cloud_resolve_use_remote');

export const cloudPullIfRemoteNewer = () =>
  invoke<string[]>('cloud_pull_if_remote_newer');

export const cloudLocalHasData = () =>
  invoke<boolean>('cloud_local_has_data');

export const cloudSetApiUrl = (url: string | null) =>
  invoke<void>('cloud_set_api_url', { url });

export const cloudGetApiUrl = () =>
  invoke<string>('cloud_get_api_url');

export const cloudLogout = () =>
  invoke<void>('cloud_logout');

export const cloudWipeRemote = () =>
  invoke<void>('cloud_wipe_remote');

export const cloudDeleteAccount = (confirmationSlug: string) =>
  invoke<void>('cloud_delete_account', { confirmationSlug });

import type { ProState } from '$lib/stores/cloud';

/** Read the current in-memory ProState from Rust. Called once at boot before
 *  the `cloud:pro-state` event subscription is set up — gives the frontend
 *  the latest known state immediately without waiting for a state-change
 *  event that may not fire on the boot path. */
export const proStateCurrent = () => invoke<ProState>('pro_state_current');

export interface InstalledSkill {
  name: string;
  path: string;
  size: number;
}

export const cloudInstallSkill = (name: string, content: string) =>
  invoke<void>('cloud_install_skill', { name, content });

export const cloudUninstallSkill = (name: string) =>
  invoke<void>('cloud_uninstall_skill', { name });

export const cloudListInstalledSkills = () =>
  invoke<InstalledSkill[]>('cloud_list_installed_skills');

export const cloudFetchMarketplaceSkills = async () => {
  const base = await getApiBaseUrl();
  return fetch(`${base}/api/marketplace/skills`).then(r => r.json());
};

export const cloudFetchSkillContent = async (url: string) => {
  const base = await getApiBaseUrl();
  return fetch(`${base}/api/marketplace/skill?url=${encodeURIComponent(url)}`).then(r => r.text());
};

async function getApiBaseUrl(): Promise<string> {
  try {
    return await cloudGetApiUrl();
  } catch {
    return 'http://67.217.243.181:3000';
  }
}
