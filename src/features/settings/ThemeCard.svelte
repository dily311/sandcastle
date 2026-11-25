<script lang="ts">
	import type { Snippet } from "svelte";
	import { _ } from "svelte-i18n";
	import type { ColorMap } from "$shared/ui/themes";

	// NAI 테마 목록처럼 바탕색·글자색을 미리 보여주는 카드
	interface Props {
		name: string;
		/** 이 카드에 쓸 색 (기본값 위에 테마 색을 덮은 것) */
		colors: ColorMap;
		selected: boolean;
		onselect: () => void;
		/** 카드 위에 겹쳐 보이는 버튼 (편집 등) */
		actions?: Snippet;
		oncontextmenu?: (e: MouseEvent) => void;
	}
	let { name, colors, selected, onselect, actions, oncontextmenu }: Props = $props();
	const c = (k: string) => colors[k];
</script>

<div class="card" class:selected {oncontextmenu} role="group" aria-label={$_("settings.more2.themeCardAria", { values: { name } })}>
	<button class="pick" role="radio" aria-checked={selected} aria-label={$_("settings.more2.themeCardApplyAria", { values: { name } })} onclick={onselect}>
		<span class="stripes" aria-hidden="true">
			<i style:background={c("--bg-sunken")}></i>
			<i style:background={c("--bg")}></i>
			<i style:background={c("--surface-2")}></i>
		</span>
		<span class="foot" style:background={c("--surface")} style:border-color={c("--border")}>
			<span class="dot" style:border-color={c("--text")} style:background={selected ? c("--accent") : "transparent"}></span>
			<b style:color={c("--text")}>{name}</b>
			<span class="sw" style:background={c("--accent")} aria-hidden="true"></span>
		</span>
	</button>
	{#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
	.card {
		position: relative;
		border-radius: var(--radius-md);
		box-shadow: 0 0 0 1px var(--border);
		overflow: hidden;
		transition: box-shadow var(--duration-fast) var(--ease);
	}
	.card:hover {
		box-shadow: 0 0 0 1px var(--border-strong), var(--shadow-sm);
	}
	.card.selected {
		box-shadow: 0 0 0 2px var(--accent);
	}
	.pick {
		display: flex;
		flex-direction: column;
		width: 100%;
		padding: 0;
		border: none;
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.stripes {
		display: flex;
		flex-direction: column;
		height: 64px;
	}
	.stripes i {
		flex: 1;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 46px;
		padding: 0 var(--space-3);
		border-top: 1px solid;
	}
	.dot {
		width: 13px;
		height: 13px;
		border-radius: 50%;
		border: 2px solid;
		flex-shrink: 0;
	}
	.foot b {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-md);
	}
	.sw {
		width: 14px;
		height: 14px;
		border-radius: 4px;
		flex-shrink: 0;
	}
	.actions {
		position: absolute;
		top: 6px;
		right: 6px;
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.card:hover .actions,
	.card:focus-within .actions {
		opacity: 1;
	}
</style>
