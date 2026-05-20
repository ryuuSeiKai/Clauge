import { writable } from 'svelte/store';

// True while a REST request is mid-drag in the collection nav. Each
// CollectionItem watches this and auto-expands its body so a drag from
// collection A can drop into collection B even if B was collapsed —
// otherwise B's `.ncoll-body` (max-height: 0) would be untargetable.
//
// Toggled by svelte-dnd-action's consider/finalize events: consider sets
// it true (fires throughout the drag), finalize sets it false (fires once
// per participating zone when the drag ends, with any trigger).
export const isDraggingRest = writable(false);
