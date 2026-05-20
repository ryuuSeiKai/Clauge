<script lang="ts">
  import type { Collection, Request } from '$lib/types';
  import { activeCollectionId, deleteCollection, updateCollection, createRequest, loadCollections, loadRequest, collectionsRefreshTrigger } from '$lib/modes/rest/stores';
  import { tabs, addTab, activateTab, closeTab } from '$lib/shared/stores/tabs';
  import { METHOD_COLORS } from '$lib/utils/theme';
  import { get } from 'svelte/store';
  import { showContextMenu } from '$lib/shared/primitives/contextmenu';
  import { showToast } from '$lib/shared/primitives/toast';
  import { friendlyError } from '$lib/utils/errors';
  import * as cmd from '$lib/commands';
  import { dndzone, type DndEvent } from 'svelte-dnd-action';
  import { isDraggingRest } from '$lib/stores/drag';
  import RequestItem from './RequestItem.svelte';
  import InlineInput from './InlineInput.svelte';
  import ConfirmDialog from '$lib/shared/primitives/ConfirmDialog.svelte';

  interface Props {
    collection: Collection;
    searchQuery?: string;
    ondeleted?: () => void;
  }

  let { collection, searchQuery = '', ondeleted }: Props = $props();

  let expanded = $state(false);
  let requests = $state<Request[]>([]);
  let loaded = $state(false);
  let addingRequest = $state(false);
  let renaming = $state(false);
  let showDeleteConfirm = $state(false);

  // Auto-expand whenever any request is being dragged anywhere — the
  // collapsed body (max-height: 0) can't receive drops, so a collapsed
  // collection would be invisible to the dndzone otherwise.
  $effect(() => {
    if ($isDraggingRest && !expanded) {
      expanded = true;
      if (!loaded) loadRequests();
    }
  });

  const isActive = $derived($activeCollectionId === collection.id);

  const filteredRequests = $derived(
    searchQuery
      ? requests.filter(r =>
          r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          r.url.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : requests
  );

  // Auto-expand when search matches requests
  $effect(() => {
    if (searchQuery && filteredRequests.length > 0 && !expanded) {
      expanded = true;
      if (!loaded) loadRequests();
    }
  });

  // Re-fetch requests when collections are refreshed (e.g. after saving a new request)
  $effect(() => {
    const _ = $collectionsRefreshTrigger;
    if (loaded) {
      cmd.listRequests(collection.id).then(r => { requests = r; }).catch(() => {});
    }
  });

  // Fetch the request list once on mount so the collection header can
  // show an accurate count BEFORE the user expands. Without this, the
  // header reads "0 requests" until first expand and only updates after
  // — confusing because the actual data is sitting on disk. The list is
  // typically small (< 100 entries per collection) so eager-loading on
  // mount has negligible cost.
  $effect(() => {
    if (!loaded) loadRequests();
  });

  async function loadRequests() {
    try {
      requests = await cmd.listRequests(collection.id);
      loaded = true;
    } catch (err) {
      // Failed to load requests — silently ignore
    }
  }

  async function toggle() {
    expanded = !expanded;
    if (expanded && !loaded) {
      await loadRequests();
    }
  }

  function buildCollMenuItems() {
    return [
      {
        label: 'Rename',
        icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>',
        action: () => { renaming = true; },
      },
      {
        label: 'Add Request',
        icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>',
        action: () => {
          expanded = true;
          if (!loaded) loadRequests();
          addingRequest = true;
        },
      },
      { label: '', action: () => {}, separator: true },
      {
        label: 'Delete',
        icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>',
        danger: true,
        action: () => { showDeleteConfirm = true; },
      },
    ];
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, buildCollMenuItems());
  }

  function handleCollMenuBtn(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    showContextMenu(rect.left, rect.bottom + 4, buildCollMenuItems());
  }

  async function handleRename(newName: string) {
    renaming = false;
    if (newName === collection.name) return;
    try {
      await updateCollection(collection.id, newName, collection.envId);
      collection.name = newName;
      showToast('Collection renamed', 'success');
    } catch (err) {
      showToast('Failed to rename collection', 'error');
    }
  }

  function cancelRename() {
    renaming = false;
  }

  function handleAddClick(e: MouseEvent) {
    e.stopPropagation();
    if (!expanded) {
      expanded = true;
      if (!loaded) loadRequests();
    }
    addingRequest = true;
  }

  async function handleAddRequest(name: string) {
    addingRequest = false;
    try {
      const req = await createRequest(collection.id, name, 'GET');
      requests = [...requests, req];

      // Auto-select the new request and open a tab
      activeCollectionId.set(collection.id);
      await loadRequest(req.id);
      const label = `GET ${name}`;
      const colors2 = METHOD_COLORS['GET'];
      addTab(label, 'rest', req.id, colors2.color);

      showToast('Request created', 'success');
    } catch (err: any) {
      showToast(friendlyError(err), 'error');
      console.error('Create request error:', err);
    }
  }

  function cancelAddRequest() {
    addingRequest = false;
  }

  async function handleDeleteCollection() {
    try {
      // Close any open topbar tabs for this collection's requests before
      // the backend deletes them, so users aren't left with orphaned tabs.
      const allTabs = get(tabs);
      for (const req of requests) {
        const tab = allTabs.find(t => t.mode === 'rest' && t.key === req.id);
        if (tab) closeTab(tab.id);
      }
      await deleteCollection(collection.id);
      showToast('Collection deleted', 'success');
      ondeleted?.();
    } catch (err) {
      showToast('Failed to delete collection', 'error');
    }
  }

  async function handleRequestChanged() {
    await loadRequests();
  }

  // Drag & drop via svelte-dnd-action — the same library Workspace uses
  // for the Kanban board. Pointer-event-based, so it sidesteps the Tauri
  // macOS issue where the native window swallows HTML5 drag events.
  //
  // `consider` fires throughout the drag for any zone whose items would
  // change — we update local state optimistically so the ghost row sits
  // where the user expects. `finalize` fires once per participating zone
  // when the drag ends; if anything in this zone now belongs to a
  // different collection, that's a cross-collection move we need to
  // persist.
  function handleConsider(e: CustomEvent<DndEvent<Request>>) {
    isDraggingRest.set(true);
    requests = e.detail.items;
  }

  async function handleFinalize(e: CustomEvent<DndEvent<Request>>) {
    requests = e.detail.items;
    isDraggingRest.set(false);

    const incoming = e.detail.items.filter(r => r.collectionId !== collection.id);
    if (incoming.length === 0) return;

    try {
      for (const req of incoming) {
        await cmd.moveRequest(req.id, collection.id);
      }
      showToast(
        incoming.length === 1 ? 'Request moved' : `${incoming.length} requests moved`,
        'success'
      );
      // loadCollections bumps collectionsRefreshTrigger, which makes every
      // CollectionItem re-fetch its requests via the existing $effect — so
      // both the source and target lists pick up the canonical state.
      await loadCollections();
    } catch (err) {
      showToast('Failed to move request', 'error');
      await loadRequests();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="ncoll">
  <div
    class="ncoll-hdr"
    class:active={isActive}
    onclick={toggle}
    oncontextmenu={handleContextMenu}
  >
    <div class="coll-icon coll-icon-accent">
      <svg viewBox="0 0 24 24"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
    </div>
    <div class="ncoll-text">
      {#if renaming}
        <InlineInput
          value={collection.name}
          placeholder="Collection name..."
          onsubmit={handleRename}
          oncancel={cancelRename}
        />
      {:else}
        <div class="ncoll-row-top">
          <span class="ncoll-name">{collection.name}</span>
        </div>
        <div class="ncoll-row-bot">
          <span class="ncoll-sub">
            {requests.length} {requests.length === 1 ? 'request' : 'requests'}
          </span>
        </div>
      {/if}
    </div>
    <button class="coll-add" title="Add request" onclick={handleAddClick}>
      <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
    </button>
    <button class="coll-menu" title="Options" onclick={handleCollMenuBtn}>
      <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor"><circle cx="12" cy="5" r="1.5"/><circle cx="12" cy="12" r="1.5"/><circle cx="12" cy="19" r="1.5"/></svg>
    </button>
    <svg class="ncoll-arr" class:open={expanded} viewBox="0 0 24 24">
      <path d="M9 18l6-6-6-6" stroke="currentColor" fill="none" stroke-width="1.8" stroke-linecap="round"/>
    </svg>
  </div>
  <div
    class="ncoll-body"
    style="max-height:{expanded ? (filteredRequests.length + (addingRequest ? 1 : 0)) * 38 + 200 + 'px' : '0'}"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="ncoll-dndzone"
      use:dndzone={{
        items: searchQuery ? filteredRequests : requests,
        type: 'rest-request',
        flipDurationMs: 150,
        dragDisabled: !!searchQuery,
        dropTargetStyle: {},
      }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each (searchQuery ? filteredRequests : requests) as req (req.id)}
        <div class="rdz-item" data-id={req.id}>
          <RequestItem request={req} ondeleted={handleRequestChanged} />
        </div>
      {/each}
    </div>
    {#if addingRequest}
      <div class="inline-add-req">
        <InlineInput
          placeholder="Request name..."
          onsubmit={handleAddRequest}
          oncancel={cancelAddRequest}
        />
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title="Delete Collection"
  message="Are you sure you want to delete '{collection.name}'? All requests in this collection will be deleted. This cannot be undone."
  confirmText="Delete"
  onconfirm={handleDeleteCollection}
/>

<style>
  .ncoll {
    border-bottom: 1px solid var(--b1);
    transition: opacity 0.15s;
  }
  .ncoll-hdr {
    min-height: 44px;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: background 0.1s;
    user-select: none;
    position: relative;
  }
  /* Two-line text block — name on top, "<n> requests · last-modified" on
     bottom. Sized to match SSH/Explorer nav rows for visual parity. */
  .ncoll-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .ncoll-row-top, .ncoll-row-bot {
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .ncoll-sub {
    font-size: 10.5px;
    font-family: var(--mono);
    color: var(--t4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ncoll-hdr.active .ncoll-sub { color: var(--t3); }
  .ncoll-hdr:hover {
    background: var(--n2);
  }
  .ncoll-hdr.active {
    background: var(--n2);
  }
  .coll-icon {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .coll-icon-accent {
    background: color-mix(in srgb, var(--acc) 15%, transparent);
    color: var(--acc);
  }
  .coll-icon svg {
    width: 13px;
    height: 13px;
    stroke: currentColor;
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
  .ncoll-name {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--t2);
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ncoll-hdr.active .ncoll-name { color: var(--t1); }
  .ncoll-arr {
    width: 12px;
    height: 12px;
    stroke: var(--t3);
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
    flex-shrink: 0;
    transition: transform 0.18s;
  }
  .ncoll-arr.open {
    transform: rotate(90deg);
  }
  .ncoll-body {
    overflow: hidden;
    background: var(--e);
    transition: max-height 0.2s ease;
  }
  .coll-add {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    border: none;
    background: transparent;
    display: none;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    color: var(--t3);
    transition: background 0.1s, color 0.1s;
  }
  .ncoll-hdr:hover .coll-add {
    display: flex;
  }
  .coll-add:hover {
    background: var(--b1);
    color: var(--t1);
  }
  .coll-add svg {
    width: 11px;
    height: 11px;
    stroke: currentColor;
    fill: none;
    stroke-width: 2.2;
    stroke-linecap: round;
  }
  .coll-menu {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    border: none;
    background: transparent;
    display: none;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    color: var(--t3);
    transition: background 0.1s, color 0.1s;
    padding: 0;
  }
  .ncoll-hdr:hover .coll-menu {
    display: flex;
  }
  .coll-menu:hover {
    background: var(--b1);
    color: var(--t1);
  }
  .inline-add-req {
    display: flex;
    gap: 4px;
    padding: 6px 10px;
    align-items: center;
    background: var(--e);
    border-bottom: 1px solid var(--b1);
  }
  .ncoll-dndzone {
    /* Acts as the dndzone target; flexbox with column layout matches the
       existing per-row stacking so svelte-dnd-action's flip animations
       align with the request rows. */
    display: flex;
    flex-direction: column;
    min-height: 4px;
  }
  .rdz-item {
    /* Suppress the library's default focus outline when an item gains
       focus during keyboard drag — looks intrusive in this dense list. */
    outline: none;
  }
</style>
