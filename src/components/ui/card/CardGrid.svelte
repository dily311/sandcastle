<script lang="ts">
    import { cn } from "$/lib/utils/cn";
    import type { HTMLAttributes } from "svelte/elements";
    import type { Snippet } from "svelte";

    type LayoutType = "horizontal" | "vertical" | "grid";

    type Props = HTMLAttributes<HTMLDivElement> & {
        layout?: LayoutType;
        columns?: number; // grid layout일 때 열 개수
        gap?: "sm" | "default" | "lg";
        children?: Snippet;
    };

    let {
        layout = "grid",
        columns = 3,
        gap = "default",
        class: className,
        children,
        style,
        ...restProps
    }: Props = $props();

    // 그리드 컬럼 동적 인라인 스타일 지정
    let gridStyle = $derived(
        layout === "grid" ? `--card-columns: repeat(${columns}, minmax(0, 1fr)); ${style || ""}` : style
    );
</script>

<div
    class={cn("card-group", layout, `gap-${gap}`, className)}
    style={gridStyle}
    {...restProps}
>
    {@render children?.()}
</div>

<style>
    /* ---------------------------------------
       카드 그룹 기본 스타일
       --------------------------------------- */
    .card-group {
        display: flex;
        width: 100%;
        box-sizing: border-box;
    }

    /* Layouts */
    .card-group.horizontal {
        flex-direction: row;
        flex-wrap: wrap;
    }

    .card-group.vertical {
        flex-direction: column;
    }

    .card-group.grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
    }

    /* Gaps */
    .card-group.gap-sm {
        gap: 0.5rem; /* 8px */
    }
    .card-group.gap-default {
        gap: 1rem; /* 16px */
    }
    .card-group.gap-lg {
        gap: 1.5rem; /* 24px */
    }

    /* 반응형 대응: 화면이 좁아지면 그리드 자동 1열 또는 2열로 조정 */
    /* @media (max-width: 768px) {
        .card-group.grid {
            grid-template-columns: repeat(1, minmax(0, 1fr)) !important;
        }
    } */
</style>