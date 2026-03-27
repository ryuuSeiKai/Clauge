<script>
  let { profiles = [], activeProfile = null, onSelect, onContextMenu, onNewSession } = $props();

  const purposeColors = {
    Brainstorming: "#d2a8ff",
    Development: "#3fb950",
    "Code Review": "#58a6ff",
    Debugging: "#f85149",
  };

  let groupedProfiles = $derived(groupByProject(profiles));

  function groupByProject(list) {
    const groups = {};
    for (const p of list) {
      const name = p.projectName || "Unknown";
      if (!groups[name]) groups[name] = [];
      groups[name].push(p);
    }
    return groups;
  }

  function relativeTime(isoString) {
    if (!isoString) return "";
    const now = Date.now();
    const then = new Date(isoString).getTime();
    const diffSec = Math.floor((now - then) / 1000);
    if (diffSec < 60) return "just now";
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 30) return `${diffDay}d ago`;
    const diffMon = Math.floor(diffDay / 30);
    return `${diffMon}mo ago`;
  }
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <span class="app-title">Clauge</span>
    <button class="new-btn" onclick={onNewSession} title="New Session">+</button>
  </div>

  <div class="sidebar-content">
    {#if profiles.length === 0}
      <div class="empty-state">
        <p>No sessions yet.</p>
        <p>Click <strong>+</strong> to create one.</p>
      </div>
    {:else}
      {#each Object.entries(groupedProfiles) as [projectName, items]}
        <div class="project-group">
          <div class="project-header">
            <svg class="folder-icon" width="14" height="14" viewBox="0 0 16 16" fill="#8b949e">
              <path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/>
            </svg>
            <span class="project-name">{projectName}</span>
          </div>
          {#each items as profile}
            <button
              class="profile-item"
              class:active={activeProfile?.id === profile.id}
              onclick={() => onSelect(profile)}
              oncontextmenu={(e) => onContextMenu(e, profile)}
            >
              <div class="profile-title">{profile.title}</div>
              <div class="profile-meta">
                <span
                  class="purpose-badge"
                  style="background: {purposeColors[profile.purpose] || '#8b949e'}22; color: {purposeColors[profile.purpose] || '#8b949e'}"
                >
                  {profile.purpose}
                </span>
                <span class="profile-time">{relativeTime(profile.lastUsedAt)}</span>
              </div>
            </button>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    min-width: 220px;
    height: 100vh;
    background: #161b22;
    border-right: 1px solid #30363d;
    display: flex;
    flex-direction: column;
    user-select: none;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 14px 12px;
    border-bottom: 1px solid #30363d;
  }

  .app-title {
    font-size: 15px;
    font-weight: 700;
    color: #e6edf3;
    letter-spacing: -0.3px;
  }

  .new-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid #30363d;
    background: transparent;
    color: #e6edf3;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, border-color 0.15s;
  }

  .new-btn:hover {
    background: #30363d;
    border-color: #58a6ff;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .sidebar-content::-webkit-scrollbar {
    width: 6px;
  }

  .sidebar-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .sidebar-content::-webkit-scrollbar-thumb {
    background: #30363d;
    border-radius: 3px;
  }

  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: #8b949e;
    font-size: 13px;
    line-height: 1.6;
  }

  .project-group {
    margin-bottom: 4px;
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px 4px;
    font-size: 11px;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .folder-icon {
    flex-shrink: 0;
  }

  .project-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-item {
    width: 100%;
    display: block;
    text-align: left;
    padding: 8px 14px;
    border: none;
    background: transparent;
    cursor: pointer;
    border-left: 3px solid transparent;
    transition: background 0.15s, border-color 0.15s;
    font-family: inherit;
  }

  .profile-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .profile-item.active {
    background: rgba(31, 111, 235, 0.2);
    border-left-color: #58a6ff;
  }

  .profile-title {
    font-size: 13px;
    font-weight: 500;
    color: #e6edf3;
    margin-bottom: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .purpose-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 10px;
    white-space: nowrap;
  }

  .profile-time {
    font-size: 11px;
    color: #8b949e;
    white-space: nowrap;
  }
</style>
