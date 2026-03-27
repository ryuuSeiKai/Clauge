<script>
  import { onMount, onDestroy } from "svelte";

  let { x = 0, y = 0, onRename, onDelete, onClose } = $props();

  let menuEl;

  function handleClickOutside(e) {
    if (menuEl && !menuEl.contains(e.target)) {
      onClose();
    }
  }

  onMount(() => {
    // Defer so the same right-click event doesn't immediately close it
    requestAnimationFrame(() => {
      document.addEventListener("click", handleClickOutside);
      document.addEventListener("contextmenu", handleClickOutside);
    });

    // Adjust position if menu overflows viewport
    if (menuEl) {
      const rect = menuEl.getBoundingClientRect();
      if (rect.right > window.innerWidth) {
        menuEl.style.left = `${window.innerWidth - rect.width - 8}px`;
      }
      if (rect.bottom > window.innerHeight) {
        menuEl.style.top = `${window.innerHeight - rect.height - 8}px`;
      }
    }
  });

  onDestroy(() => {
    document.removeEventListener("click", handleClickOutside);
    document.removeEventListener("contextmenu", handleClickOutside);
  });
</script>

<div class="context-menu" style="left: {x}px; top: {y}px;" bind:this={menuEl}>
  <button class="menu-item" onclick={onRename}>
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
      <path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25a1.75 1.75 0 01.445-.758l8.61-8.61zm1.414 1.06a.25.25 0 00-.354 0L3.46 11.1a.25.25 0 00-.064.108l-.631 2.208 2.208-.63a.25.25 0 00.108-.064l8.61-8.61a.25.25 0 000-.354l-1.086-1.086z"/>
    </svg>
    Rename
  </button>
  <button class="menu-item menu-item-danger" onclick={onDelete}>
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
      <path d="M6.5 1.75a.25.25 0 01.25-.25h2.5a.25.25 0 01.25.25V3h-3V1.75zM11 3V1.75A1.75 1.75 0 009.25 0h-2.5A1.75 1.75 0 005 1.75V3H2.75a.75.75 0 000 1.5h.928l.747 10.218A1.75 1.75 0 006.172 16h3.656a1.75 1.75 0 001.747-1.282L12.322 4.5h.928a.75.75 0 000-1.5H11zm-5.522 1.5l.735 10.06a.25.25 0 00.249.19h3.076a.25.25 0 00.249-.19l.735-10.06H5.478z"/>
    </svg>
    Delete
  </button>
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 2000;
    background: #1c2128;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 4px;
    min-width: 140px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    background: transparent;
    color: #e6edf3;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    border-radius: 5px;
    transition: background 0.12s;
  }

  .menu-item:hover {
    background: #30363d;
  }

  .menu-item-danger:hover {
    background: rgba(248, 81, 73, 0.15);
    color: #f85149;
  }
</style>
