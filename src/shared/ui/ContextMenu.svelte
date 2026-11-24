<script lang="ts" module>
	import type { Component } from "svelte";
	export interface MenuItem {
		label: string;
		icon?: Component;
		danger?: boolean;
		disabled?: boolean;
		/** 앞에 구분선 */
		separator?: boolean;
		/** 보일 조건 */
		visible?: boolean;
		/** 선택 표시 (정렬 등 하나를 고르는 메뉴) */
		checked?: boolean;
		/** 같은 이름이 여럿일 수 있을 때 항목을 구분하는 키 (기본은 이름). 이름이 같은 캐릭터 목록처럼 */
		id?: string;
		/** 오른쪽에 보이는 지금 값 (하위 메뉴가 있을 때: 그룹화 기준 … 폴더) */
		value?: string;
		/** 하위 메뉴 (한 단계). 있으면 눌러서(마우스는 올려서) 옆에 펼침 — onselect는 쓰지 않음 */
		children?: MenuItem[];
		onselect?: () => void;
	}
</script>

<script lang="ts">
	/**
	 * 오른쪽 클릭 메뉴. 사용:
	 *   let menu = $state<{ x: number; y: number } | null>(null);
	 *   <div oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}>
	 *   <ContextMenu at={menu} items={...} onclose={() => (menu = null)} />
	 */
	interface Props {
		/** align "end": x가 메뉴의 오른쪽 끝 (버튼 오른쪽 끝에 맞춰 버튼 아래에 띄울 때). 메뉴 실제 폭으로 맞춘다 */
		at: { x: number; y: number; align?: "start" | "end" } | null;
		items: MenuItem[];
		label?: string;
		onclose: () => void;
	}
	let { at, items, label = "메뉴", onclose }: Props = $props();

	let el: HTMLDivElement | undefined = $state();
	let pos = $state({ x: 0, y: 0 });
	/** 펼친 하위 메뉴 (항목 키) */
	let openSub = $state<string | null>(null);
	/** 좁은 화면: 하위 메뉴를 옆이 아니라 항목 아래에 펼침 */
	let inline = $state(false);
	$effect(() => {
		if (!at) openSub = null;
		else inline = window.innerWidth < 640;
	});
	const keyOf = (it: MenuItem) => it.id ?? it.label;
	/** 메뉴를 body로 옮겨 그린다: 서랍처럼 transform이 걸린 조상 안에서는 position: fixed가 화면이 아니라 그 조상을 기준으로 잡혀 메뉴가 화면 밖에 뜸.
	 * 열린 <dialog>(그림 크게 보기·대화 상자) 안이면 그 dialog로 옮긴다 — 모달 dialog는 맨 위 층이라 body의 메뉴는 z-index와 상관없이 뒤에 묻히고 누를 수도 없다 */
	function portal(node: HTMLElement) {
		const host = node.parentElement?.closest("dialog[open]") ?? document.body;
		host.appendChild(node);
		return { destroy: () => node.remove() };
	}
	/** 하위 메뉴가 화면 밖으로 나가면 왼쪽으로 뒤집고, 아래로 넘치면 위로 올린다 */
	function placeSub(node: HTMLElement, side: boolean) {
		if (!side) return;
		const r = node.getBoundingClientRect();
		if (r.right > window.innerWidth - 8) node.classList.add("flip");
		const over = r.bottom - (window.innerHeight - 8);
		if (over > 0) node.style.top = `${-4 - over}px`;
	}

	// 화면 밖으로 나가지 않게 위치 보정, 첫 항목에 포커스
	$effect(() => {
		if (!at || !el) return;
		const r = el.getBoundingClientRect();
		const left = at.align === "end" ? at.x - r.width : at.x;
		pos = {
			x: Math.max(8, Math.min(left, window.innerWidth - r.width - 8)),
			y: Math.min(at.y, window.innerHeight - r.height - 8),
		};
		el.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
	});

	function onkeydown(e: KeyboardEvent) {
		if (e.key === "Escape") return onclose();
		if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
		e.preventDefault();
		const buttons = [...(el?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
		const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
		buttons[(i + (e.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length]?.focus();
	}
</script>

<svelte:window onmousedown={(e) => at && el && !el.contains(e.target as Node) && onclose()} onblur={() => at && onclose()} />

{#if at}
	<div bind:this={el} use:portal class="menu" class:scroll={inline} role="menu" aria-label={label} tabindex="-1" style:left="{pos.x}px" style:top="{pos.y}px" {onkeydown}>
		{#each items as it}
			{#if it.visible === undefined || it.visible}
				{#if it.separator}<div class="sep" role="separator"></div>{/if}
				{#if it.children}
					<div class="has-sub" role="none" onmouseenter={() => !inline && !it.disabled && (openSub = keyOf(it))}>
						<button role="menuitem" aria-haspopup="menu" aria-expanded={openSub === keyOf(it)} disabled={it.disabled} onclick={() => (openSub = openSub === keyOf(it) ? null : keyOf(it))}>
							{#if it.icon}<it.icon />{/if}<span class="lbl">{it.label}</span>
							{#if it.value}<span class="val">{it.value}</span>{/if}<span class="arrow" class:open={inline && openSub === keyOf(it)} aria-hidden="true">›</span>
						</button>
						{#if openSub === keyOf(it)}
							<div class={inline ? "inline-sub" : "menu sub"} role="menu" aria-label={it.label} use:placeSub={!inline}>
								{#each it.children as ch (keyOf(ch))}
									{#if it.visible === undefined || it.visible}
										{#if ch.separator}<div class="sep" role="separator"></div>{/if}
										<button
											role={ch.checked === undefined ? "menuitem" : "menuitemradio"}
											aria-checked={ch.checked}
											class:danger={ch.danger}
											disabled={ch.disabled}
											onclick={() => {
												ch.onselect?.();
												onclose();
											}}
										>
											{#if ch.icon}<ch.icon />{/if}<span class="lbl">{ch.label}</span>
											{#if ch.checked}<span class="check" aria-hidden="true">✓</span>{/if}
										</button>
									{/if}
								{/each}
							</div>
						{/if}
					</div>
				{:else}
					<button
						role={it.checked === undefined ? "menuitem" : "menuitemradio"}
						aria-checked={it.checked}
						class:danger={it.danger}
						disabled={it.disabled}
						onmouseenter={() => (openSub = null)}
						onclick={() => {
							it.onselect?.();
							onclose();
						}}
					>
						{#if it.icon}<it.icon />{/if}<span class="lbl">{it.label}</span>
						{#if it.checked}<span class="check" aria-hidden="true">✓</span>{/if}
					</button>
				{/if}
			{/if}
		{/each}
	</div>
{/if}

<style>
	.menu {
		position: fixed;
		z-index: var(--z-menu);
		min-width: min(168px, calc(100vw - 16px));
		max-width: min(340px, calc(100vw - 16px));
		max-height: 100%;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		outline: none;
		animation: in var(--duration-fast) var(--ease);
	}
	@keyframes in {
		from {
			opacity: 0;
			transform: scale(0.97);
		}
	}
	button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	button :global(svg) {
		width: 15px;
		height: 15px;
		color: var(--text-muted);
	}
	button:hover:not(:disabled),
	button:focus-visible {
		background: var(--surface-hover);
		outline: none;
	}
	button:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.danger,
	.danger :global(svg) {
		color: var(--danger);
	}
	.check {
		margin-left: auto;
		padding-left: var(--space-3);
		color: var(--accent-text);
		font-weight: 700;
	}
	/* 좁은 화면은 메뉴가 길어지면 스크롤 (옆 펼침이 없으니 안 잘림) */
	.menu.scroll {
		max-height: calc(100dvh - 16px);
		overflow-y: auto;
	}
	/* 이름은 한 줄, 길면 … */
	.lbl {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.has-sub {
		position: relative;
	}
	.inline-sub {
		padding: 0 0 2px var(--space-3);
	}
	/* 하위 메뉴: 항목 오른쪽에 붙어 펼침 (화면 밖이면 왼쪽으로 뒤집힘) */
	.menu.sub {
		position: absolute;
		z-index: 1;
		top: -5px;
		left: calc(100% + 2px);
		max-width: min(280px, calc(100vw - 16px));
		max-height: calc(100dvh - 16px);
		overflow-y: auto;
		animation: none;
	}
	.menu.sub:global(.flip) {
		left: auto;
		right: calc(100% + 2px);
	}
	.val {
		flex: none;
		max-width: 45%;
		margin-left: auto;
		padding-left: var(--space-4);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.arrow.open {
		transform: rotate(90deg);
	}
	.val + .arrow {
		margin-left: var(--space-1);
	}
	.arrow {
		flex: none;
		display: inline-block;
		transition: transform var(--duration-fast) var(--ease);
		margin-left: auto;
		color: var(--text-faint);
		font-size: var(--text-md);
		line-height: 1;
	}
	.sep {
		height: 1px;
		margin: 4px 2px;
		background: var(--border);
	}
</style>
