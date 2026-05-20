<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import { showToast } from "$lib/shared/primitives/toast";
    import { highlightJSON } from "$lib/shared/utils/json-highlight";
    import ResponseHeaders from "./ResponseHeaders.svelte";
    import { onMount, onDestroy } from "svelte";

    interface Props {
        response: HttpResponse | null;
        loading: boolean;
    }

    let { response, loading }: Props = $props();

    type TabId = "pretty" | "raw" | "preview" | "headers";
    const TABS: { id: TabId; label: string }[] = [
        { id: "pretty", label: "Pretty" },
        { id: "raw", label: "Raw" },
        { id: "preview", label: "Preview" },
        { id: "headers", label: "Headers" },
    ];

    let activeTab = $state<TabId>("pretty");

    // Search state
    let showSearch = $state(false);
    let searchQuery = $state("");
    let searchInputRef = $state<HTMLInputElement | null>(null);
    let currentMatchIndex = $state(0);
    let viewerRef = $state<HTMLDivElement | null>(null);

    function escapeRegex(s: string): string {
        return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    }

    const matchCount = $derived.by(() => {
        if (!searchQuery || !response) return 0;
        const body =
            activeTab === "pretty"
                ? prettyBody
                : activeTab === "raw"
                  ? response.body
                  : "";
        if (!body) return 0;
        const regex = new RegExp(escapeRegex(searchQuery), "gi");
        const matches = body.match(regex);
        return matches ? matches.length : 0;
    });

    const searchHighlightedBody = $derived.by(() => {
        if (!searchQuery || !response) return "";
        const body =
            activeTab === "pretty"
                ? prettyBody
                : activeTab === "raw"
                  ? response.body
                  : "";
        if (!body) return "";
        // For pretty tab, apply JSON highlighting first, then search highlight
        if (activeTab === "pretty") {
            const highlighted = highlightJSON(body);
            if (!searchQuery) return highlighted;
            // We need to highlight matches in the visible text, not in HTML tags
            return highlightSearchInHtml(
                highlighted,
                searchQuery,
                currentMatchIndex,
            );
        }
        // For raw tab, escape HTML then highlight search
        return highlightSearchInText(body, searchQuery, currentMatchIndex);
    });

    function escapeHtml(str: string): string {
        return str
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
    }

    function highlightSearchInText(
        text: string,
        query: string,
        activeIndex: number,
    ): string {
        const escaped = escapeHtml(text);
        const regex = new RegExp(escapeRegex(escapeHtml(query)), "gi");
        let matchIdx = 0;
        return escaped.replace(regex, (match) => {
            const cls =
                matchIdx === activeIndex
                    ? "search-match active-match"
                    : "search-match";
            matchIdx++;
            return `<mark class="${cls}">${match}</mark>`;
        });
    }

    function highlightSearchInHtml(
        html: string,
        query: string,
        activeIndex: number,
    ): string {
        // Split HTML into tags and text segments, only highlight in text segments
        const parts = html.split(/(<[^>]*>)/);
        const escapedQuery = escapeRegex(query);
        const regex = new RegExp(escapedQuery, "gi");
        let matchIdx = 0;

        return parts
            .map((part) => {
                if (part.startsWith("<")) return part;
                return part.replace(regex, (match) => {
                    const cls =
                        matchIdx === activeIndex
                            ? "search-match active-match"
                            : "search-match";
                    matchIdx++;
                    return `<mark class="${cls}">${match}</mark>`;
                });
            })
            .join("");
    }

    function formatSize(bytes: number): string {
        if (bytes < 1024) return bytes + " B";
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
        return (bytes / (1024 * 1024)).toFixed(1) + " MB";
    }

    function formatBody(body: string): string {
        try {
            return JSON.stringify(JSON.parse(body), null, 2);
        } catch {
            return body;
        }
    }

    // ── Binary / SSL detection ──────────────────────────────────────────

    function getContentType(headers: [string, string][]): string {
        return (
            headers
                .find(([k]) => k.toLowerCase() === "content-type")?.[1]
                ?.toLowerCase() ?? ""
        );
    }

    function isBinaryMimeType(ct: string): boolean {
        const mime = ct.split(";")[0].trim();
        if (!mime) return false;
        if (mime.startsWith("text/")) return false;
        if (
            [
                "application/json",
                "application/xml",
                "application/javascript",
                "application/x-www-form-urlencoded",
                "application/ld+json",
                "application/graphql+json",
                "application/graphql",
                "application/x-yaml",
                "application/yaml",
                "application/atom+xml",
                "application/rss+xml",
            ].includes(mime)
        )
            return false;
        if (
            mime.startsWith("image/") ||
            mime.startsWith("audio/") ||
            mime.startsWith("video/") ||
            mime.startsWith("font/")
        )
            return true;
        if (
            [
                "application/octet-stream",
                "application/pdf",
                "application/zip",
                "application/gzip",
                "application/x-gzip",
                "application/wasm",
                "application/x-tar",
                "application/x-bzip2",
                "application/x-rar-compressed",
                "application/x-7z-compressed",
                "application/x-msdownload",
            ].includes(mime)
        )
            return true;
        if (
            mime.startsWith("application/vnd.") &&
            !mime.includes("+xml") &&
            !mime.includes("+json")
        )
            return true;
        return false;
    }

    const contentType = $derived(
        response ? getContentType(response.headers) : "",
    );

    // Null bytes in the body are a reliable fallback indicator of binary data
    const isBinary = $derived(
        response
            ? isBinaryMimeType(contentType) || response.body.includes("\x00")
            : false,
    );

    const isSslError = $derived(
        !!response &&
            response.status === 0 &&
            /^ssl-error:/i.test(response.body),
    );

    const sslReason = $derived(
        isSslError
            ? response!.body.replace(/^ssl-error:\s*/i, "") ||
                  "Certificate verification failed"
            : "",
    );

    // ── Display values ──────────────────────────────────────────────────

    const isSuccess = $derived(
        response ? response.status >= 200 && response.status < 300 : false,
    );

    // Return '' for binary/SSL so search & copy fall back to raw body cleanly
    const prettyBody = $derived(
        response && !isBinary && !isSslError ? formatBody(response.body) : "",
    );
    const highlightedBody = $derived(
        prettyBody ? highlightJSON(prettyBody) : "",
    );

    async function copyResponse() {
        if (!response) return;
        // Copy whatever the user is currently viewing, not always the body.
        let text: string;
        let label: string;
        if (activeTab === "headers") {
            text = response.headers.map(([k, v]) => `${k}: ${v}`).join("\n");
            label = "Headers copied";
        } else if (activeTab === "pretty") {
            text = prettyBody;
            label = "Response copied";
        } else {
            text = response.body;
            label = "Response copied";
        }
        try {
            await writeText(text);
            showToast(label, "success");
        } catch {
            showToast("Failed to copy", "error");
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if ((e.metaKey || e.ctrlKey) && e.key === "f" && response) {
            e.preventDefault();
            showSearch = true;
            setTimeout(() => searchInputRef?.focus(), 0);
        }
    }

    function handleSearchKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            closeSearch();
        } else if (e.key === "Enter") {
            if (e.shiftKey) {
                prevMatch();
            } else {
                nextMatch();
            }
        }
    }

    function nextMatch() {
        if (matchCount === 0) return;
        currentMatchIndex = (currentMatchIndex + 1) % matchCount;
        scrollToMatch();
    }

    function prevMatch() {
        if (matchCount === 0) return;
        currentMatchIndex = (currentMatchIndex - 1 + matchCount) % matchCount;
        scrollToMatch();
    }

    function scrollToMatch() {
        setTimeout(() => {
            const active = viewerRef?.querySelector(".active-match");
            if (active) {
                active.scrollIntoView({ block: "center", behavior: "smooth" });
            }
        }, 10);
    }

    function closeSearch() {
        showSearch = false;
        searchQuery = "";
        currentMatchIndex = 0;
    }

    // Reset match index when query changes
    $effect(() => {
        if (searchQuery) {
            currentMatchIndex = 0;
        }
    });

    onMount(() => {
        document.addEventListener("keydown", handleKeydown);
    });

    onDestroy(() => {
        document.removeEventListener("keydown", handleKeydown);
    });
</script>

<div class="response-viewer">
    {#if !response && !loading}
        <!-- Empty state -->
        <div class="empty">
            <div class="empty-icon">
                <svg viewBox="0 0 24 24">
                    <circle cx="12" cy="12" r="10" />
                    <path d="M8 12l3 3 5-5" />
                </svg>
            </div>
            <div class="empty-text">
                Press <span class="kbd">&#8984;Enter</span> or click
                <span class="kbd">Send</span>
                to send request<br />
                <span class="kbd">&#8984;L</span> to toggle AI Assistant
            </div>
        </div>
    {:else if loading}
        <!-- Loading state -->
        <div class="empty">
            <div class="loading-text">
                Sending request<span class="loading-dots"></span>
            </div>
        </div>
    {:else if response}
        <!-- Response header bar -->
        <div class="resp-hdr">
            <span
                class="status-pill"
                class:s-ok={isSuccess}
                class:s-err={!isSuccess}
            >
                {response.status}
                {response.status_text}
            </span>
            <span class="resp-meta">{response.duration_ms}ms</span>
            <span class="resp-meta">{formatSize(response.size_bytes)}</span>
            <div class="resp-spacer"></div>
            <button
                class="resp-copy"
                onclick={copyResponse}
                title="Copy response body"
            >
                <svg viewBox="0 0 24 24" width="13" height="13">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                    <path
                        d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"
                    />
                </svg>
            </button>
        </div>

        <!-- Search bar -->
        {#if showSearch}
            <div class="search-bar">
                <div class="search-icon">
                    <svg viewBox="0 0 24 24" width="13" height="13"
                        ><circle
                            cx="11"
                            cy="11"
                            r="8"
                            stroke="currentColor"
                            fill="none"
                            stroke-width="2"
                        /><path
                            d="M21 21l-4.35-4.35"
                            stroke="currentColor"
                            fill="none"
                            stroke-width="2"
                            stroke-linecap="round"
                        /></svg
                    >
                </div>
                <input
                    bind:this={searchInputRef}
                    class="search-input"
                    type="text"
                    placeholder="Search response..."
                    bind:value={searchQuery}
                    onkeydown={handleSearchKeydown}
                />
                {#if searchQuery}
                    <span class="search-count"
                        >{matchCount > 0
                            ? `${currentMatchIndex + 1} of ${matchCount}`
                            : "No matches"}</span
                    >
                {/if}
                <button
                    class="search-nav-btn"
                    onclick={prevMatch}
                    title="Previous (Shift+Enter)"
                    disabled={matchCount === 0}
                >
                    <svg viewBox="0 0 24 24" width="12" height="12"
                        ><path
                            d="M18 15l-6-6-6 6"
                            stroke="currentColor"
                            fill="none"
                            stroke-width="2"
                            stroke-linecap="round"
                        /></svg
                    >
                </button>
                <button
                    class="search-nav-btn"
                    onclick={nextMatch}
                    title="Next (Enter)"
                    disabled={matchCount === 0}
                >
                    <svg viewBox="0 0 24 24" width="12" height="12"
                        ><path
                            d="M6 9l6 6 6-6"
                            stroke="currentColor"
                            fill="none"
                            stroke-width="2"
                            stroke-linecap="round"
                        /></svg
                    >
                </button>
                <button
                    class="search-nav-btn"
                    onclick={closeSearch}
                    title="Close (Esc)"
                >
                    <svg viewBox="0 0 24 24" width="12" height="12"
                        ><path
                            d="M18 6L6 18M6 6l12 12"
                            stroke="currentColor"
                            fill="none"
                            stroke-width="2"
                            stroke-linecap="round"
                        /></svg
                    >
                </button>
            </div>
        {/if}

        <!-- Tab bar -->
        <div class="ph">
            {#each TABS as tab (tab.id)}
                <button
                    class="pht"
                    class:on={activeTab === tab.id}
                    onclick={() => {
                        activeTab = tab.id;
                    }}
                >
                    {tab.label}
                </button>
            {/each}
        </div>

        <!-- Tab content -->
        {#if activeTab === "pretty"}
            <div class="viewer" bind:this={viewerRef}>
                {#if isSslError}
                    <div class="inline-guide ssl-guide">
                        <div class="ig-head">
                            <svg
                                viewBox="0 0 24 24"
                                width="18"
                                height="18"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.8"
                                stroke-linecap="round"
                                ><rect
                                    x="3"
                                    y="11"
                                    width="18"
                                    height="11"
                                    rx="2"
                                /><path d="M7 11V7a5 5 0 0110 0v4" /></svg
                            >
                            <span>SSL certificate verification failed</span>
                        </div>
                        <p class="ig-reason">{sslReason}</p>
                        <p class="ig-text">
                            To call APIs with self-signed or untrusted
                            certificates, disable SSL verification in Settings:
                        </p>
                        <ol class="ig-steps">
                            <li>Open <strong>Settings</strong></li>
                            <li>Select the <strong>REST</strong> tab</li>
                            <li>
                                Toggle off <strong>SSL Verification</strong>
                            </li>
                        </ol>
                        <div class="ig-warn">
                            <svg
                                viewBox="0 0 24 24"
                                width="13"
                                height="13"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                ><path
                                    d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                /><line x1="12" y1="9" x2="12" y2="13" /><line
                                    x1="12"
                                    y1="17"
                                    x2="12.01"
                                    y2="17"
                                /></svg
                            >
                            Only disable SSL verification for trusted internal or
                            test APIs.
                        </div>
                    </div>
                {:else if isBinary}
                    <div class="inline-guide binary-guide">
                        <div class="ig-head">
                            <svg
                                viewBox="0 0 24 24"
                                width="18"
                                height="18"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.8"
                                stroke-linecap="round"
                                ><path
                                    d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z"
                                /><polyline points="13 2 13 9 20 9" /></svg
                            >
                            <span>Binary content</span>
                        </div>
                        <p class="ig-text">
                            {contentType || "application/octet-stream"} &middot; {formatSize(
                                response.size_bytes,
                            )}
                        </p>
                        <p class="ig-hint">
                            Switch to <strong>Raw</strong> to view the response bytes
                            as text.
                        </p>
                    </div>
                {:else if searchQuery}
                    {@html searchHighlightedBody}
                {:else}
                    {@html highlightedBody}
                {/if}
            </div>
        {:else if activeTab === "raw"}
            <div class="viewer raw" bind:this={viewerRef}>
                {#if isBinary}
                    <div class="raw-binary-note">
                        Binary response — bytes rendered as UTF-8 (may appear
                        garbled)
                    </div>
                {/if}
                {#if searchQuery}
                    {@html searchHighlightedBody}
                {:else}
                    {response.body}
                {/if}
            </div>
        {:else if activeTab === "preview"}
            <div class="viewer preview-msg">
                <span>Preview not available for JSON responses</span>
            </div>
        {:else if activeTab === "headers"}
            <ResponseHeaders headers={response.headers} />
        {/if}
    {/if}
</div>

<style>
    .response-viewer {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }

    /* ── Empty state ── */
    .empty {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 10px;
        background: transparent;
    }

    .empty-icon svg {
        width: 36px;
        height: 36px;
        stroke: var(--t4);
        fill: none;
        stroke-width: 1.2;
        stroke-linecap: round;
    }

    .empty-text {
        font-size: 12px;
        color: var(--t3);
        text-align: center;
        line-height: 1.6;
    }

    .kbd {
        display: inline-block;
        padding: 1px 5px;
        border-radius: 3px;
        border: 1px solid var(--b1);
        font-size: 10px;
        font-family: var(--mono);
        color: var(--t3);
        background: var(--n);
    }

    /* ── Loading ── */
    .loading-text {
        font-size: 12px;
        color: var(--t3);
        font-family: var(--mono);
    }

    .loading-dots::after {
        content: "";
        animation: dots 1.4s steps(4, end) infinite;
    }

    @keyframes dots {
        0% {
            content: "";
        }
        25% {
            content: ".";
        }
        50% {
            content: "..";
        }
        75% {
            content: "...";
        }
        100% {
            content: "";
        }
    }

    /* ── Response header bar ── */
    .resp-hdr {
        height: 32px;
        flex-shrink: 0;
        background: transparent;
        border-bottom: 1px solid var(--b1);
        padding: 0 12px;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .status-pill {
        font-size: 12px;
        font-weight: 600;
        padding: 2px 9px;
        border-radius: 10px;
        font-family: var(--mono);
    }

    .s-ok {
        background: rgba(29, 200, 128, 0.1);
        color: var(--ok);
    }

    .s-err {
        background: rgba(240, 68, 68, 0.1);
        color: var(--err);
    }

    .resp-meta {
        font-size: 11px;
        color: var(--t3);
        font-family: var(--mono);
    }

    .resp-spacer {
        flex: 1;
    }

    .resp-copy {
        width: 24px;
        height: 24px;
        border: none;
        background: transparent;
        color: var(--t3);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
        transition:
            color 0.1s,
            background 0.1s;
    }
    .resp-copy:hover {
        color: var(--t1);
        background: var(--b1);
    }
    .resp-copy svg {
        stroke: currentColor;
        fill: none;
        stroke-width: 1.8;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    /* ── Search bar ── */
    .search-bar {
        height: 34px;
        flex-shrink: 0;
        background: var(--n2);
        border-bottom: 1px solid var(--b1);
        padding: 0 10px;
        display: flex;
        align-items: center;
        gap: 6px;
        animation: slideDown 0.15s ease;
    }
    @keyframes slideDown {
        from {
            opacity: 0;
            transform: translateY(-4px);
        }
        to {
            opacity: 1;
            transform: none;
        }
    }
    .search-icon {
        color: var(--t3);
        display: flex;
        align-items: center;
        flex-shrink: 0;
    }
    .search-input {
        flex: 1;
        background: transparent;
        border: 1px solid var(--b1);
        border-radius: 4px;
        padding: 7px 11px;
        font-size: 12px;
        font-family: var(--mono);
        color: var(--t1);
        outline: none;
        height: 24px;
        transition: border-color 0.15s;
    }
    .search-input:focus {
        border-color: var(--acc);
    }
    .search-input::placeholder {
        color: var(--t3);
    }
    .search-count {
        font-size: 10.5px;
        color: var(--t3);
        font-family: var(--mono);
        white-space: nowrap;
        flex-shrink: 0;
    }
    .search-nav-btn {
        width: 22px;
        height: 22px;
        border: none;
        background: transparent;
        color: var(--t3);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
        flex-shrink: 0;
        transition:
            color 0.1s,
            background 0.1s;
    }
    .search-nav-btn:hover:not(:disabled) {
        color: var(--t1);
        background: var(--b1);
    }
    .search-nav-btn:disabled {
        opacity: 0.3;
        cursor: default;
    }

    /* ── Tab bar ── */
    .ph {
        height: 34px;
        flex-shrink: 0;
        background: var(--n2);
        border-bottom: 1px solid var(--b1);
        padding: 0 12px;
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .pht {
        font-size: 12px;
        color: var(--t3);
        cursor: pointer;
        font-family: var(--mono);
        padding-bottom: 2px;
        border: none;
        background: transparent;
        border-bottom: 1.5px solid transparent;
        transition: color 0.1s;
    }
    .pht.on {
        color: var(--t1);
        border-bottom-color: var(--acc);
    }
    .pht:hover:not(.on) {
        color: var(--t2);
    }

    /* ── Viewer ── */
    .viewer {
        flex: 1;
        min-height: 0;
        background: transparent;
        padding: 12px 14px;
        overflow-y: auto;
        overflow-x: hidden;
        font-family: var(--mono);
        font-size: 12.5px;
        line-height: 1.75;
        white-space: pre-wrap;
        word-break: break-word;
        color: var(--t1);
    }
    .viewer::-webkit-scrollbar {
        width: 4px;
    }
    .viewer::-webkit-scrollbar-thumb {
        background: var(--b1);
        border-radius: 2px;
    }

    .raw {
        white-space: pre-wrap;
        word-break: break-all;
    }

    .preview-msg {
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--t3);
        font-size: 12px;
        white-space: normal;
    }

    /* ── Inline guides (SSL / binary) ── */
    .inline-guide {
        padding: 20px 18px;
        border-radius: 8px;
        border: 1px solid var(--b1);
        margin: 4px 0;
        display: flex;
        flex-direction: column;
        gap: 10px;
        max-width: 560px;
        /* Reset .viewer body-text styles that would otherwise leak in:
           pre-wrap turns the indented JSX source into visible whitespace,
           break-word splits long URLs mid-glyph, and the mono font makes
           the prose look like a code dump. */
        white-space: normal;
        word-break: normal;
        overflow-wrap: anywhere;
        line-height: 1.5;
        font-family: var(--ui);
    }
    .ssl-guide {
        border-color: color-mix(in srgb, var(--warn, #f5a623) 30%, var(--b1));
        background: color-mix(in srgb, var(--warn, #f5a623) 5%, transparent);
    }
    .binary-guide {
        background: color-mix(in srgb, var(--acc) 4%, transparent);
    }
    .ig-head {
        display: flex;
        align-items: center;
        gap: 8px;
        font-weight: 600;
        font-size: 13px;
        color: var(--t1);
    }
    .ssl-guide .ig-head {
        color: var(--warn, #f5a623);
    }
    .ig-reason {
        font-size: 11.5px;
        font-family: var(--mono);
        color: var(--t3);
        margin: 0;
        word-break: break-all;
    }
    .ig-text {
        font-size: 12.5px;
        color: var(--t2);
        margin: 0;
        line-height: 1.5;
    }
    .ig-hint {
        font-size: 12px;
        color: var(--t3);
        margin: 0;
    }
    .ig-steps {
        margin: 0;
        padding-left: 20px;
        font-size: 12.5px;
        color: var(--t2);
        display: flex;
        flex-direction: column;
        gap: 5px;
    }
    .ig-steps strong {
        color: var(--t1);
    }
    .ig-warn {
        display: flex;
        align-items: flex-start;
        gap: 6px;
        font-size: 11.5px;
        color: var(--t3);
        padding: 8px 10px;
        background: color-mix(in srgb, var(--warn, #f5a623) 8%, transparent);
        border-radius: 5px;
        line-height: 1.4;
    }
    .ig-warn svg {
        flex-shrink: 0;
        color: var(--warn, #f5a623);
        margin-top: 1px;
    }
    .raw-binary-note {
        font-size: 11px;
        font-family: var(--mono);
        color: var(--t4);
        padding: 4px 8px;
        margin-bottom: 8px;
        border-left: 2px solid var(--b1);
        line-height: 1.4;
    }

    /* Search match highlighting */
    :global(.search-match) {
        background: rgba(245, 166, 35, 0.25);
        color: inherit;
        border-radius: 2px;
        padding: 0 1px;
    }
    :global(.active-match) {
        background: rgba(245, 166, 35, 0.55);
        outline: 1px solid rgba(245, 166, 35, 0.6);
    }
</style>
