import { invoke } from '@tauri-apps/api/core';

export const editorGetPort = () => invoke<number>('editor_get_port');
export const editorOpenProject = (projectPath: string) => invoke<number>('editor_open_project', { projectPath });
export const editorSetBinaryPath = (path: string) => invoke<void>('editor_set_binary_path', { path });
export const editorSyncTheme = (themeJson: string) => invoke<void>('editor_sync_theme', { themeJson });
