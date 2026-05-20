<script lang="ts">
    import {
        welcomeProModalOpen,
        welcomeProPlanHint,
        postCheckoutVerifying,
        cloudSub,
    } from "$lib/stores/cloud";
    import { openSettingsTab } from "$lib/shared/stores/tabs";

    function close() {
        if ($postCheckoutVerifying) return; // can't dismiss during verification
        welcomeProModalOpen.set(false);
        welcomeProPlanHint.set(null);
    }

    function openManage() {
        if ($postCheckoutVerifying) return;
        welcomeProModalOpen.set(false);
        welcomeProPlanHint.set(null);
        openSettingsTab("account");
    }

    function teleportToBody(node: HTMLElement) {
        document.body.appendChild(node);
        return {
            destroy() {
                if (node.parentElement === document.body) node.remove();
            },
        };
    }

    // Plan resolution: prefer live API value, fall back to URL hint so we
    // still render the right tier when the webhook hasn't reached D1 yet.
    let isLifetime = $derived(
        $cloudSub?.isLifetime === true ||
            ($cloudSub?.interval == null && $welcomeProPlanHint === "lifetime"),
    );
    let interval = $derived(
        $cloudSub?.interval ?? $welcomeProPlanHint ?? null,
    );

    let tierLabel = $derived(
        isLifetime
            ? "Lifetime"
            : interval === "yearly"
              ? "Yearly"
              : interval === "monthly"
                ? "Monthly"
                : null,
    );

    // Subtitle for the celebration state. Plan-aware, only the "forever"
    // claim is gated on lifetime — recurring users see a neutral message.
    let subtitle = $derived(
        isLifetime
            ? "Pro is unlocked forever. Everything is ready to go."
            : "Pro is unlocked. Everything is ready to go.",
    );

    // Verifying-state subtitle. Uses the URL hint so we can name the tier
    // even before /api/auth/me catches up. Falls back to generic copy.
    let verifyingSubtitle = $derived(
        $welcomeProPlanHint === "lifetime"
            ? "Activating Pro Lifetime — your one-time purchase is confirming with our servers."
            : $welcomeProPlanHint === "yearly"
              ? "Activating Pro Yearly — your subscription is confirming with our servers."
              : $welcomeProPlanHint === "monthly"
                ? "Activating Pro Monthly — your subscription is confirming with our servers."
                : "Activating Pro — your purchase is confirming with our servers.",
    );
</script>

{#if $welcomeProModalOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="overlay" onclick={close} use:teleportToBody>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
            class="modal"
            class:is-lifetime={isLifetime}
            class:is-verifying={$postCheckoutVerifying}
            onclick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-labelledby="welcome-pro-title"
            aria-busy={$postCheckoutVerifying}
        >
            {#if $postCheckoutVerifying}
                <!-- ── Verifying state: spinner + status ──────────────── -->
                <div class="spinner-wrap" aria-hidden="true">
                    <span class="spinner"></span>
                </div>

                <h2 id="welcome-pro-title" class="title">
                    Confirming your purchase
                </h2>

                <p class="sub">{verifyingSubtitle}</p>

                <p class="hint">This usually takes a few seconds.</p>
            {:else}
                <!-- ── Celebration state: confirmed Pro ───────────────── -->
                <div class="badge" aria-hidden="true">
                    <svg width="26" height="26" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 2l2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6L12 2z" />
                    </svg>
                </div>

                <h2 id="welcome-pro-title" class="title">Welcome to Clauge Pro</h2>

                {#if tierLabel}
                    <span class="tier-pill">{tierLabel}</span>
                {/if}

                <p class="sub">{subtitle}</p>

                <button class="cta" onclick={close}>Continue</button>
                <button class="link" onclick={openManage}>Manage subscription</button>
            {/if}
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.68);
        backdrop-filter: blur(14px) saturate(140%);
        z-index: 9999;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 1rem;
        animation: wpm-fade 180ms ease-out;
    }

    .modal {
        /* Always uses the user's current theme accent — lifetime, monthly,
         * and yearly all share the same hue so the modal feels native to
         * whatever palette the user has on. Tier identity comes from the
         * pill label, not the color. */
        --plan-body: var(--acc, #c2185b);
        --plan-text: var(--acc, #c2185b);
        --plan-grad: linear-gradient(
            120deg,
            color-mix(in srgb, var(--plan-body) 100%, transparent),
            color-mix(in srgb, var(--plan-body) 70%, #fff)
        );

        position: relative;
        width: 100%;
        max-width: 420px;
        padding: 38px 32px 24px;
        border-radius: 16px;
        background: var(--n2, #0e0e0e);
        box-shadow:
            0 24px 60px rgba(0, 0, 0, 0.55),
            0 0 0 1px rgba(255, 255, 255, 0.06) inset,
            0 0 0 1px color-mix(in srgb, var(--plan-body) 22%, transparent);
        text-align: center;
        color: var(--t1, #ddd);
        font-family: var(--ui);
        animation: wpm-pop 220ms cubic-bezier(0.22, 1, 0.36, 1);
    }
    .modal.is-verifying {
        padding-bottom: 30px;
    }

    .modal::before {
        content: "";
        position: absolute;
        inset: -1px;
        border-radius: inherit;
        background: radial-gradient(
            60% 50% at 50% 0%,
            color-mix(in srgb, var(--plan-body) 18%, transparent) 0%,
            transparent 70%
        );
        pointer-events: none;
        z-index: 0;
    }

    .badge,
    .spinner-wrap,
    .title,
    .tier-pill,
    .sub,
    .hint,
    .cta,
    .link {
        position: relative;
        z-index: 1;
    }

    /* ── Verifying state ─────────────────────────────────────────── */
    .spinner-wrap {
        width: 56px;
        height: 56px;
        margin: 4px auto 22px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
    .spinner {
        width: 36px;
        height: 36px;
        border-radius: 999px;
        border: 2.5px solid
            color-mix(in srgb, var(--plan-body) 22%, transparent);
        border-top-color: var(--plan-text);
        animation: wpm-spin 0.8s linear infinite;
    }
    .hint {
        margin: 8px 0 0;
        font-size: 11.5px;
        color: var(--t4, var(--t3, #888));
        opacity: 0.85;
    }

    /* ── Celebration state ───────────────────────────────────────── */
    .badge {
        width: 56px;
        height: 56px;
        margin: 0 auto 18px;
        border-radius: 999px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        color: var(--plan-text);
        background: color-mix(in srgb, var(--plan-body) 14%, transparent);
        border: 1px solid color-mix(in srgb, var(--plan-body) 30%, transparent);
    }
    .badge svg {
        filter: drop-shadow(
            0 0 8px color-mix(in srgb, var(--plan-body) 50%, transparent)
        );
    }

    .title {
        margin: 0 0 10px;
        font-size: 22px;
        font-weight: 700;
        letter-spacing: -0.02em;
        color: var(--t1, #fff);
    }

    .tier-pill {
        display: inline-block;
        margin: 0 auto 14px;
        padding: 3px 10px;
        font-family: var(--mono, ui-monospace);
        font-size: 10.5px;
        font-weight: 700;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--plan-text);
        background: color-mix(in srgb, var(--plan-body) 14%, transparent);
        border: 1px solid color-mix(in srgb, var(--plan-body) 30%, transparent);
        border-radius: 999px;
    }

    .sub {
        margin: 0 auto 24px;
        max-width: 340px;
        font-size: 13.5px;
        line-height: 1.55;
        color: var(--t2, #aaa);
    }
    .modal.is-verifying .sub {
        margin-bottom: 0;
    }

    .cta {
        appearance: none;
        border: 0;
        cursor: pointer;
        width: 100%;
        padding: 12px 20px;
        border-radius: 10px;
        background: var(--plan-grad);
        color: #fff;
        font-family: inherit;
        font-size: 13.5px;
        font-weight: 600;
        letter-spacing: -0.005em;
        transition:
            transform 0.16s ease,
            filter 0.16s ease;
        box-shadow: 0 8px 22px
            color-mix(in srgb, var(--plan-body) 28%, transparent);
    }
    .cta:hover {
        transform: translateY(-1px);
        filter: brightness(1.08);
    }

    .link {
        appearance: none;
        background: transparent;
        border: 0;
        cursor: pointer;
        width: 100%;
        margin-top: 10px;
        padding: 6px;
        color: var(--t3, #888);
        font-family: inherit;
        font-size: 12px;
        font-weight: 500;
        transition: color 0.12s;
    }
    .link:hover {
        color: var(--t1, #ddd);
    }

    @keyframes wpm-fade {
        from { opacity: 0; }
        to { opacity: 1; }
    }
    @keyframes wpm-pop {
        from { opacity: 0; transform: scale(0.96); }
        to { opacity: 1; transform: scale(1); }
    }
    @keyframes wpm-spin {
        to { transform: rotate(360deg); }
    }
</style>
