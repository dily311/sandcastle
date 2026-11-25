<script lang="ts">
	// 프로필 고르기: 지금 고른 프로필(아바타 + 이름)을 누르면 목록이 뜬다.
	// 대화(페르소나), 게임(플레이어), 폰(주인) 패널이 같은 모양을 쓴다.
	import { UserCog } from "@lucide/svelte";
	import Avatar from "./Avatar.svelte";
	import type { AvatarFit } from "$ipc/bindings";
	import { fileSrc } from "$shared/ipc/file";

	export interface ProfileOption {
		id: string;
		name: string;
		avatar?: string | null;
		/** 프로필 그림 확대·위치 (5-E-17-3) */
		avatarFit?: AvatarFit | null;
		color?: string | null;
		description?: string | null;
		/** 목록을 나눌 제목 (예: "페르소나", "캐릭터") */
		group?: string;
	}

	let {
		options,
		value,
		label = "프로필 바꾸기",
		hint,
		onchange,
		onmanage,
		manageLabel = "추가 · 관리",
	}: {
		options: ProfileOption[];
		value: string | null | undefined;
		label?: string;
		/** 고른 프로필 이름 아래에 설명 대신 보여줄 글 */
		hint?: string;
		onchange: (id: string) => void;
		/** 하단 추가·관리 동작 (주면 팝오버 맨 아래에 버튼 표시) */
		onmanage?: () => void;
		manageLabel?: string;
	} = $props();

	const current = $derived(options.find((o) => o.id === value) ?? null);
	let open = $state(false);
	let button: HTMLButtonElement | undefined = $state();
	let popStyle = $state("");

	function toggle() {
		open = !open;
		if (!open || !button) return;
		// 패널 스크롤에 잘리지 않게 버튼 기준 고정 위치. 아래 공간이 넉넉하면 아래로, 아니면 위로 연다
		const r = button.getBoundingClientRect();
		const below = window.innerHeight - r.bottom;
		popStyle =
			below > 260 || below > r.top
				? `left:${r.left}px;width:${r.width}px;top:${r.bottom + 6}px;max-height:${below - 12}px`
				: `left:${r.left}px;width:${r.width}px;bottom:${window.innerHeight - r.top + 6}px;max-height:${r.top - 12}px`;
	}
	function pick(id: string) {
		open = false;
		if (id !== value) onchange(id);
	}
	const groups = $derived.by(() => {
		const out: { title?: string; items: ProfileOption[] }[] = [];
		for (const o of options) {
			const last = out[out.length - 1];
			if (last && last.title === o.group) last.items.push(o);
			else out.push({ title: o.group, items: [o] });
		}
		return out;
	});
</script>

<svelte:window
	onmousedown={(e) => {
		const t = e.target as Element;
		if (open && !t.closest(".ppop") && !t.closest(".pcur")) open = false;
	}}
	onkeydown={(e) => e.key === "Escape" && (open = false)}
/>

{#if open}
	<div class="ppop" role="listbox" aria-label={label} style={popStyle}>
		{#each groups as g, gi (gi)}
			{#if g.title}<p class="grp">{g.title}</p>{/if}
			{#each g.items as o (o.id)}
				<button role="option" aria-selected={o.id === value} class:on={o.id === value} onclick={() => pick(o.id)}>
					<Avatar name={o.name} src={fileSrc(o.avatar)} fit={o.avatarFit} color={o.color || null} size={28} />
					<span class="pt"><b>{o.name}</b>{#if o.description}<small>{o.description}</small>{/if}</span>
				</button>
			{/each}
		{/each}
		{#if onmanage}
			<div class="pfoot">
				<button
					type="button"
					class="manage"
					onclick={() => {
						open = false;
						onmanage();
					}}
				>
					<UserCog />
					<span>{manageLabel}</span>
				</button>
			</div>
		{/if}
	</div>
{/if}
<button bind:this={button} class="pcur" aria-haspopup="listbox" aria-expanded={open} aria-label={label} onclick={toggle}>
	<Avatar name={current?.name ?? "?"} src={fileSrc(current?.avatar)} fit={current?.avatarFit} color={current?.color || null} size={40} />
	<span class="pt">
		<b>{current?.name ?? "없음"}</b>
		{#if hint}<small>{hint}</small>{:else if current?.description}<small>{current.description}</small>{/if}
	</span>
</button>

<style>
	.pcur,
	.ppop button {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.pcur {
		border: 1px solid var(--border);
		background: var(--surface);
	}
	.pcur:hover,
	.ppop button:hover {
		background: var(--surface-hover);
	}
	.pt {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		line-height: 1.3;
	}
	.pt small {
		color: var(--text-muted);
		font-size: var(--text-xs);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.ppop {
		position: fixed;
		z-index: var(--z-popover);
		overflow: auto;
		display: flex;
		flex-direction: column;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		backdrop-filter: blur(14px);
		-webkit-backdrop-filter: blur(14px);
		box-shadow: var(--shadow-lg);
		animation: up var(--duration-fast) var(--ease);
	}
	@keyframes up {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	.ppop button.on {
		background: var(--accent-soft);
	}
	.pfoot {
		margin-top: 4px;
		padding-top: 4px;
		border-top: 1px solid var(--border);
	}
	.ppop button.manage {
		gap: var(--space-2);
		padding: 7px var(--space-2);
		color: var(--text-muted);
		font-size: var(--text-xs);
		font-weight: 500;
	}
	.ppop button.manage:hover {
		color: var(--text);
	}
	.ppop button.manage :global(svg) {
		width: 14px;
		height: 14px;
		flex: none;
	}
	.grp {
		margin: 6px 8px 2px;
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
</style>
