import { invoke } from '@tauri-apps/api/core';
import type {
  AgentSession,
  AgentContext,
  DiscoveredSession,
  ContextUsage,
  GitFileChange,
  UsageAnalytics,
  ClaudePlugin,
  MarketplacePlugin,
} from '$lib/types/agent';

// Session commands
export const agentListSessions = () => invoke<AgentSession[]>('agent_list_sessions');
export const agentCreateSession = (session: Omit<AgentSession, 'id' | 'createdAt' | 'lastUsedAt'>) => invoke<AgentSession>('agent_create_session', { session });
export const agentUpdateSession = (session: AgentSession) => invoke<void>('agent_update_session', { session });
export const agentDeleteSession = (id: string) => invoke<void>('agent_delete_session', { id });
export const agentUpdateSessionId = (id: string, claudeSessionId: string) => invoke<void>('agent_update_session_id', { id, claudeSessionId });
export const agentUpdateLastUsed = (id: string) => invoke<void>('agent_update_last_used', { id });
export const agentUpdateWorktree = (id: string, worktreePath: string | null, worktreeBranch: string | null) => invoke<void>('agent_update_worktree', { id, worktreePath, worktreeBranch });

// Context commands
export const agentListContexts = () => invoke<AgentContext[]>('agent_list_contexts');
export const agentSaveContext = (name: string, content: string) => invoke<AgentContext>('agent_save_context', { name, content });
export const agentDeleteContext = (id: string) => invoke<void>('agent_delete_context', { id });
export const agentGetSessionContexts = (sessionId: string) => invoke<AgentContext[]>('agent_get_session_contexts', { sessionId });
export const agentAttachContext = (sessionId: string, contextId: string) => invoke<void>('agent_attach_context', { sessionId, contextId });
export const agentDetachContext = (sessionId: string, contextId: string) => invoke<void>('agent_detach_context', { sessionId, contextId });

// Terminal commands
export const agentSpawnTerminal = (sessionId: string, workDir: string, onOutput: any) => invoke<string>('agent_spawn_terminal', { sessionId, workDir, onOutput });
export const agentSpawnShell = (workDir: string, onOutput: any) => invoke<string>('agent_spawn_shell', { workDir, onOutput });
export const agentWriteToTerminal = (terminalId: string, data: string) => invoke<void>('agent_write_to_terminal', { terminalId, data });
export const agentResizeTerminal = (terminalId: string, cols: number, rows: number) => invoke<void>('agent_resize_terminal', { terminalId, cols, rows });
export const agentKillTerminal = (terminalId: string) => invoke<void>('agent_kill_terminal', { terminalId });

// Worktree commands
export const agentIsGitRepo = (path: string) => invoke<boolean>('agent_is_git_repo', { path });
export const agentCreateWorktree = (repoPath: string, branch: string) => invoke<string>('agent_create_worktree', { repoPath, branch });
export const agentRemoveWorktree = (worktreePath: string) => invoke<void>('agent_remove_worktree', { worktreePath });

// Git commands
export const agentGitStatus = (repoPath: string) => invoke<GitFileChange[]>('agent_git_status', { repoPath });
export const agentGitBranch = (repoPath: string) => invoke<string>('agent_git_branch', { repoPath });
export const agentGitAheadBehind = (repoPath: string) => invoke<[number, number]>('agent_git_ahead_behind', { repoPath });
export const agentGitCommit = (repoPath: string, message: string) => invoke<string>('agent_git_commit', { repoPath, message });
export const agentGitPush = (repoPath: string) => invoke<void>('agent_git_push', { repoPath });
export const agentGitPull = (repoPath: string) => invoke<void>('agent_git_pull', { repoPath });
export const agentGitDiffFile = (repoPath: string, filePath: string) => invoke<string>('agent_git_diff_file', { repoPath, filePath });
export const agentGitStageFile = (repoPath: string, filePath: string) => invoke<void>('agent_git_stage_file', { repoPath, filePath });
export const agentGitUnstageFile = (repoPath: string, filePath: string) => invoke<void>('agent_git_unstage_file', { repoPath, filePath });
export const agentGitLog = (repoPath: string, limit: number) => invoke<string[]>('agent_git_log', { repoPath, limit });
export const agentGitStash = (repoPath: string) => invoke<void>('agent_git_stash', { repoPath });
export const agentGitStashPop = (repoPath: string) => invoke<void>('agent_git_stash_pop', { repoPath });
export const agentGitListBranches = (repoPath: string) => invoke<string[]>('agent_git_list_branches', { repoPath });
export const agentGitSwitchBranch = (repoPath: string, branch: string) => invoke<void>('agent_git_switch_branch', { repoPath, branch });

// Plugin commands
export const agentGetPlugins = () => invoke<ClaudePlugin[]>('agent_get_plugins');
export const agentTogglePlugin = (name: string, enabled: boolean) => invoke<void>('agent_toggle_plugin', { name, enabled });
export const agentGetMarketplacePlugins = () => invoke<MarketplacePlugin[]>('agent_get_marketplace_plugins');
export const agentInstallPlugin = (name: string, marketplace: string) => invoke<void>('agent_install_plugin', { name, marketplace });
export const agentUninstallPlugin = (name: string) => invoke<void>('agent_uninstall_plugin', { name });

// Usage commands
export const agentGetUsageAnalytics = (days: number) => invoke<UsageAnalytics>('agent_get_usage_analytics', { days });
export const agentFetchUsageLimits = () => invoke<Record<string, unknown>>('agent_fetch_usage_limits');
export const agentDiscoverSessions = (projectPath: string) => invoke<DiscoveredSession[]>('agent_discover_sessions', { projectPath });
export const agentGetSessionTokens = (sessionId: string) => invoke<number>('agent_get_session_tokens', { sessionId });
export const agentGetSessionContextUsage = (sessionId: string) => invoke<ContextUsage>('agent_get_session_context_usage', { sessionId });
