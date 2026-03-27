<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";

  let { terminalId = null, terminalError = null } = $props();

  let containerEl;
  let term = null;
  let fitAddon = null;
  let unlisten = null;
  let currentTerminalId = null;
  let resizeObserver = null;
  let termReady = false;

  function ensureTerminal() {
    if (termReady || !containerEl) return;

    term = new Terminal({
      theme: {
        background: "#0d1117",
        foreground: "#e6edf3",
        cursor: "#58a6ff",
        cursorAccent: "#0d1117",
        selectionBackground: "rgba(88, 166, 255, 0.3)",
        black: "#484f58",
        red: "#ff7b72",
        green: "#3fb950",
        yellow: "#d29922",
        blue: "#58a6ff",
        magenta: "#bc8cff",
        cyan: "#39d353",
        white: "#b1bac4",
        brightBlack: "#6e7681",
        brightRed: "#ffa198",
        brightGreen: "#56d364",
        brightYellow: "#e3b341",
        brightBlue: "#79c0ff",
        brightMagenta: "#d2a8ff",
        brightCyan: "#56d364",
        brightWhite: "#f0f6fc",
      },
      fontFamily: '"SF Mono", "Fira Code", "Cascadia Code", monospace',
      fontSize: 13,
      lineHeight: 1.4,
      cursorBlink: true,
      cursorStyle: "bar",
      scrollback: 10000,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerEl);

    term.onData((data) => {
      if (currentTerminalId) {
        invoke("write_to_terminal", { terminalId: currentTerminalId, data }).catch(
          (e) => console.error("write_to_terminal failed:", e)
        );
      }
    });

    term.onResize(({ cols, rows }) => {
      if (currentTerminalId) {
        invoke("resize_terminal", { terminalId: currentTerminalId, cols, rows }).catch(
          (e) => console.error("resize_terminal failed:", e)
        );
      }
    });

    resizeObserver = new ResizeObserver(() => {
      if (fitAddon && term && containerEl.offsetWidth > 0) {
        requestAnimationFrame(() => {
          try { fitAddon.fit(); } catch (_) {}
        });
      }
    });
    resizeObserver.observe(containerEl);

    termReady = true;
  }

  async function attachToTerminal(tid) {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }

    currentTerminalId = tid;

    if (term) {
      term.clear();
      term.reset();
    }

    // Make container visible and fit
    containerEl.style.display = "block";
    requestAnimationFrame(() => {
      if (fitAddon) {
        try { fitAddon.fit(); } catch(_) {}
      }
    });

    unlisten = await listen("terminal-output", (event) => {
      const { terminal_id, data } = event.payload;
      if (terminal_id === currentTerminalId && term) {
        try {
          const binary = atob(data);
          const bytes = new Uint8Array(binary.length);
          for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
          }
          term.write(bytes);
        } catch (e) {
          console.error("Failed to decode terminal data:", e);
        }
      }
    });

    // Send initial size after fit
    setTimeout(() => {
      if (fitAddon && currentTerminalId) {
        try {
          fitAddon.fit();
          const dims = fitAddon.proposeDimensions();
          if (dims) {
            invoke("resize_terminal", {
              terminalId: currentTerminalId,
              cols: dims.cols,
              rows: dims.rows,
            }).catch(() => {});
          }
        } catch(_) {}
      }
    }, 100);
  }

  // React to terminalId changes
  let prevTerminalId = null;
  $effect(() => {
    const tid = terminalId;
    if (tid && tid !== prevTerminalId) {
      prevTerminalId = tid;
      // Use setTimeout to escape Svelte's effect tracking
      setTimeout(() => {
        ensureTerminal();
        if (termReady) {
          attachToTerminal(tid);
        }
      }, 0);
    }
    if (!tid) {
      prevTerminalId = null;
      if (containerEl) {
        containerEl.style.display = "none";
      }
    }
  });

  onMount(() => {
    // containerEl is bound by now
    if (containerEl) {
      containerEl.style.display = "none";
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (resizeObserver) resizeObserver.disconnect();
    if (term) term.dispose();
  });
</script>

<div class="terminal-panel">
  {#if terminalError && !terminalId}
    <div class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#f85149" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="15" y1="9" x2="9" y2="15"></line>
          <line x1="9" y1="9" x2="15" y2="15"></line>
        </svg>
      </div>
      <p class="empty-title" style="color: #f85149;">Failed to start terminal</p>
      <p class="empty-subtitle" style="max-width: 500px; word-break: break-word;">{terminalError}</p>
    </div>
  {:else if !terminalId}
    <div class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#30363d" stroke-width="1.5">
          <polyline points="4 17 10 11 4 5"></polyline>
          <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
      </div>
      <p class="empty-title">No active session</p>
      <p class="empty-subtitle">Select a session from the sidebar or create a new one</p>
    </div>
  {/if}
  <div class="terminal-container" bind:this={containerEl}></div>
</div>

<style>
  .terminal-panel {
    flex: 1;
    min-width: 0;
    height: 100vh;
    background: #0d1117;
    position: relative;
    overflow: hidden;
  }

  .terminal-container {
    width: 100%;
    height: 100%;
    padding: 4px;
  }

  .terminal-container :global(.xterm) {
    height: 100%;
  }

  .terminal-container :global(.xterm-viewport) {
    overflow-y: auto !important;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar) {
    width: 8px;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: transparent;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: #30363d;
    border-radius: 4px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #8b949e;
    gap: 12px;
    position: absolute;
    inset: 0;
  }

  .empty-icon {
    margin-bottom: 8px;
    opacity: 0.6;
  }

  .empty-title {
    font-size: 16px;
    font-weight: 500;
    color: #e6edf3;
  }

  .empty-subtitle {
    font-size: 13px;
    color: #8b949e;
  }
</style>
