import { writable } from 'svelte/store';

export const editorPort = writable<number | null>(null);
export const editorProjectPath = writable<string>('');
