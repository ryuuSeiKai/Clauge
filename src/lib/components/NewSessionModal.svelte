<script>
  import { open } from "@tauri-apps/plugin-dialog";

  let { onClose, onCreate } = $props();

  let projectPath = $state("");
  let title = $state("");
  let purpose = $state("");

  const purposes = [
    { label: "Brainstorming", color: "#d2a8ff" },
    { label: "Development", color: "#3fb950" },
    { label: "Code Review", color: "#58a6ff" },
    { label: "Debugging", color: "#f85149" },
  ];

  let canCreate = $derived(projectPath.trim() !== "" && title.trim() !== "" && purpose !== "");

  async function browsePath() {
    try {
      const selected = await open({ directory: true, multiple: false, title: "Select Project Folder" });
      if (selected) {
        projectPath = selected;
        if (!title) {
          title = selected.split("/").filter(Boolean).pop() || "";
        }
      }
    } catch (e) {
      console.error("Folder picker failed:", e);
    }
  }

  function handleCreate() {
    if (!canCreate) return;
    onCreate({ projectPath: projectPath.trim(), title: title.trim(), purpose });
  }

  function handleKeydown(e) {
    if (e.key === "Escape") onClose();
  }

  function handleBackdropClick(e) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdropClick}>
  <div class="modal">
    <div class="modal-header">
      <h2>New Session</h2>
    </div>

    <div class="modal-body">
      <div class="field">
        <label for="project-path">Project Folder</label>
        <div class="path-input-row">
          <input
            id="project-path"
            type="text"
            bind:value={projectPath}
            placeholder="/path/to/project"
            class="text-input path-input"
          />
          <button class="browse-btn" onclick={browsePath}>Browse</button>
        </div>
      </div>

      <div class="field">
        <label for="session-title">Session Title</label>
        <input
          id="session-title"
          type="text"
          bind:value={title}
          placeholder="e.g., Fix auth bug"
          class="text-input"
        />
      </div>

      <div class="field">
        <label>Purpose</label>
        <div class="purpose-chips">
          {#each purposes as p}
            <button
              class="purpose-chip"
              class:selected={purpose === p.label}
              style={purpose === p.label
                ? `background: ${p.color}22; color: ${p.color}; border-color: ${p.color}`
                : ""}
              onclick={() => (purpose = p.label)}
            >
              {p.label}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="modal-footer">
      <button class="btn btn-cancel" onclick={onClose}>Cancel</button>
      <button class="btn btn-create" disabled={!canCreate} onclick={handleCreate}>Create</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    width: 440px;
    max-width: 90vw;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid #30363d;
  }

  .modal-header h2 {
    font-size: 15px;
    font-weight: 600;
    color: #e6edf3;
    margin: 0;
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field label {
    font-size: 12px;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .text-input {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 13px;
    color: #e6edf3;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }

  .text-input:focus {
    border-color: #58a6ff;
  }

  .text-input::placeholder {
    color: #484f58;
  }

  .path-input-row {
    display: flex;
    gap: 8px;
  }

  .path-input {
    flex: 1;
    min-width: 0;
  }

  .browse-btn {
    background: #21262d;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 8px 14px;
    font-size: 13px;
    color: #e6edf3;
    cursor: pointer;
    white-space: nowrap;
    font-family: inherit;
    transition: background 0.15s, border-color 0.15s;
  }

  .browse-btn:hover {
    background: #30363d;
    border-color: #58a6ff;
  }

  .purpose-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .purpose-chip {
    padding: 6px 14px;
    border-radius: 16px;
    border: 1px solid #30363d;
    background: transparent;
    color: #8b949e;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s;
  }

  .purpose-chip:hover {
    border-color: #8b949e;
    color: #e6edf3;
  }

  .purpose-chip.selected {
    font-weight: 600;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid #30363d;
  }

  .btn {
    padding: 7px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    border: 1px solid transparent;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn-cancel {
    background: #21262d;
    border-color: #30363d;
    color: #e6edf3;
  }

  .btn-cancel:hover {
    background: #30363d;
  }

  .btn-create {
    background: #238636;
    color: #ffffff;
    border-color: rgba(240, 246, 252, 0.1);
  }

  .btn-create:hover:not(:disabled) {
    background: #2ea043;
  }

  .btn-create:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
