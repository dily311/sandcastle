<script lang="ts">
    import { cn } from "$/lib/utils/cn";
    import type { HTMLAttributes, HTMLAnchorAttributes } from "svelte/elements";
    import type { Snippet } from "svelte";

    type Props = (HTMLAttributes<HTMLDivElement> & HTMLAnchorAttributes) & {
        href?: string;
        variant?: "default" | "outline" | "ghost";
        padding?: "none" | "sm" | "default" | "lg";
        children?: Snippet;
    };

    let {
        href,
        variant = "default",
        padding = "default",
        class: className,
        children,
        ...restProps
    }: Props = $props();
</script>

{#if href}
    <a
        {href}
        class={cn("card", variant, padding !== "default" && `p-${padding}`, className)}
        {...restProps}
    >
        {@render children?.()}
    </a>
{:else}
    <div
        class={cn("card", variant, padding !== "default" && `p-${padding}`, className)}
        {...restProps}
    >
        {@render children?.()}
    </div>
{/if}

<style>
    /* ---------------------------------------
       카드 기본 스타일
       --------------------------------------- */
    .card {
        display: flex;
        flex-direction: column;
		gap: 0.8rem;
        background: var(--card-bg, var(--background, #ffffff));
        color: var(--text-color);
        border-radius: var(--radius-md, 8px);
        border: 1px solid var(--border-color);
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.02);
        padding: 1.25rem; /* 기본 패딩 (20px) */
        transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease;
        text-decoration: none;
        box-sizing: border-box;
    }

    /* 링크형 카드일 때 호버 효과 */
    a.card:hover {
        border-color: var(--border-strong, #d1d5db);
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05);
    }

    /* Variants */
    .card.outline {
        background: transparent;
    }

    .card.ghost {
        background: transparent;
        border-color: transparent;
        box-shadow: none;
    }

    /* Paddings */
    .card.p-none {
        padding: 0;
    }
    .card.p-sm {
        padding: 0.75rem; /* 12px */
    }
    .card.p-lg {
        padding: 2rem; /* 32px */
    }

    /* 카드 내부 서브 컴포넌트 편의 스타일링 전역 선택자 */
    :global(.card > header) {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin-bottom: 0.75rem;
    }

    :global(.card > footer) {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 0.5rem;
        margin-top: 1rem;
        padding-top: 0.75rem;
        border-top: 1px solid var(--border-color);
    }
</style>