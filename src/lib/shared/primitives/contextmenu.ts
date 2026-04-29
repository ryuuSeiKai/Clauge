import { writable } from 'svelte/store';

export interface ContextMenuItem {
  label: string;
  icon?: string;
  action: () => void;
  danger?: boolean;
  separator?: boolean;
}

export interface ContextMenuState {
  show: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export const contextMenu = writable<ContextMenuState>({
  show: false,
  x: 0,
  y: 0,
  items: []
});

export function showContextMenu(x: number, y: number, items: ContextMenuItem[]): void {
  contextMenu.set({ show: true, x, y, items });
}

export function closeContextMenu(): void {
  contextMenu.update((s) => ({ ...s, show: false }));
}
