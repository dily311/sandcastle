<script lang="ts">
	// 내보내기 한 버튼 + 옆 화살표로 형식 고르기 (고른 형식은 이 컴퓨터에 기억)
	import { Check, ChevronDown, Download } from "@lucide/svelte";
	import { Button, ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { convActions } from "./actions";
	import type { Conversation } from "./library.svelte";

	let { conv }: { conv: Conversation } = $props();

	type Format = "jsonl" | "txt" | "zip";
	const FORMATS: { id: Format; label: string; desc: string }[] = [
		{ id: "jsonl", label: "SillyTavern 채팅 (.jsonl)", desc: "SillyTavern에서 바로 불러올 수 있음" },
		{ id: "txt", label: "텍스트 (.txt)", desc: "읽기용. 이름과 대사만" },
		{ id: "zip", label: "캐릭터 묶음 (.zip)", desc: "캐릭터·이미지와 함께, 샌드캐슬끼리 옮길 때" },
	];
	const KEY = "sc.export.format";
	let format = $state<Format>(
		(() => {
			try {
				const v = localStorage.getItem(KEY);
				return v === "txt" || v === "zip" ? v : "jsonl";
			} catch {
				return "jsonl";
			}
		})(),
	);
	const current = $derived(FORMATS.find((f) => f.id === format) ?? FORMATS[0]);

	let menu = $state<{ x: number; y: number } | null>(null);
	let btn: HTMLDivElement | undefined = $state();
	const items = $derived<MenuItem[]>(
		FORMATS.map((f) => ({
			label: f.label,
			icon: f.id === format ? Check : undefined,
			onselect: () => {
				format = f.id;
				try {
					localStorage.setItem(KEY, f.id);
				} catch {
					/* 기억 못 해도 그만 */
				}
			},
		})),
	);
	function openMenu() {
		const r = btn?.getBoundingClientRect();
		if (r) menu = { x: r.left, y: r.bottom + 4 };
	}
	function run() {
		if (format === "zip") void convActions.export(conv);
		else void convActions.exportAs(conv, format);
	}
</script>

<div class="split" bind:this={btn}>
	<Button variant="primary" onclick={run}>
		{#snippet icon()}<Download />{/snippet}
		파일로 내보내기
	</Button>
	<IconButton label="내보낼 형식 고르기" variant="soft" aria-haspopup="menu" aria-expanded={!!menu} onclick={openMenu}><ChevronDown /></IconButton>
</div>
<p class="fmt">{current.label} · {current.desc}</p>
<ContextMenu at={menu} label="내보낼 형식" {items} onclose={() => (menu = null)} />

<style>
	.split {
		display: flex;
		gap: 2px;
	}
	.split > :global(:first-child) {
		flex: 1;
		border-top-right-radius: 0;
		border-bottom-right-radius: 0;
	}
	.split > :global(:last-child) {
		border-top-left-radius: 0;
		border-bottom-left-radius: 0;
	}
	.fmt {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
</style>
