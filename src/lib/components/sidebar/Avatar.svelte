<script lang="ts">
  import { cloudConnected, cloudUser, cloudDisplayHandle, cloudConflicts } from '$lib/stores/cloud';

  // The avatar tile itself indicates "signed in" (filled image / letters
  // vs. the unconnected "CL" placeholder), so a green dot here would be
  // redundant. We only show a dot when the user has something to *do* —
  // currently: one or more kinds are conflict-locked and need their pick.
  const needsAttention = $derived($cloudConnected && $cloudConflicts.length > 0);

  // When the avatar URL fails to load (offline, host blocked, image
  // expired, etc.) the <img> would render the broken-image glyph. Flip
  // a flag on `onerror` so we fall back to the initial-letter view
  // instead. Reset whenever the URL changes so a retry after the
  // network recovers is possible.
  let imgFailed = $state(false);
  $effect(() => {
    // Track the URL so the effect re-runs when it changes.
    const _ = $cloudUser?.avatarUrl;
    imgFailed = false;
  });
</script>

<div class="avatar-connected">
  <button
    class="avatar"
    title={needsAttention
      ? `Action required — ${$cloudConflicts.length} item${$cloudConflicts.length === 1 ? '' : 's'} to resolve`
      : ($cloudConnected ? `${$cloudDisplayHandle?.handle ?? ''}` : 'Profile')}
  >
    {#if $cloudConnected && $cloudUser?.avatarUrl && !imgFailed}
      <img
        class="avatar-img"
        src={$cloudUser.avatarUrl}
        alt={$cloudDisplayHandle?.handle ?? ''}
        referrerpolicy="no-referrer"
        onerror={() => (imgFailed = true)}
      />
    {:else if $cloudConnected}
      <span class="avatar-letter">{($cloudDisplayHandle?.handle ?? 'U').charAt(0).toUpperCase()}</span>
    {:else}
      <span class="avatar-letter">CL</span>
    {/if}
  </button>
  {#if needsAttention}
    <span class="avatar-dot avatar-dot-action" aria-label="Action required"></span>
  {/if}
</div>

<style>
  .avatar-connected {
    position: relative;
  }
  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--acc), var(--acc));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    color: #fff;
    cursor: default;
    margin-top: 6px;
    border: none;
    transition: opacity 0.15s;
    font-family: var(--ui);
    overflow: hidden;
    padding: 0;
  }
  .avatar:hover {
    opacity: 0.85;
  }
  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
  }
  .avatar-letter {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
  }
  .avatar-dot {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid var(--s);
  }
  /* "Action required" — accent fill + soft pulse so the user notices
     they have something to resolve even from across the screen. */
  .avatar-dot-action {
    background: var(--acc);
    animation: avatarActionPulse 1.6s ease-in-out infinite;
  }
  @keyframes avatarActionPulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--acc) 50%, transparent); }
    50%      { box-shadow: 0 0 0 4px color-mix(in srgb, var(--acc) 0%, transparent); }
  }
</style>
