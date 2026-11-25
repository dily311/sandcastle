<script lang="ts" generics="T extends string">
	import { _ } from "svelte-i18n";
	// 내보내기 한 버튼 + 옆 화살표로 형식 고르기 (고른 형식은 storageKey로 이 컴퓨터에 기억)
	import { Check, ChevronDown, Download } from "@lucide/svelte";
	import Button from "./Button.svelte";
	import IconButton from "./IconButton.svelte";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
	import { rememberedFormat, rememberFormat, type ExportFormat } from "$app/exportFormats.svelte";

	let {
		formats,
		storageKey,
		onexport,
		label,
		size = "md",
		variant = "primary",
		hint = true,
	}: {
		formats: ExportFormat<T>[];
		storageKey: string;
		onexport: (id: T) => void;
		label?: string;
		size?: "sm" | "md";
		variant?: "primary" | "secondary" | "ghost";
		/** 버튼 아래 고른 형식 설명 */
		hint?: boolean;
	} = $props();
	const effectiveLabel = $derived(label ?? $_("sharedControls.exportFileDefault"));

	// svelte-ignore state_referenced_locally
	let format = $state<T>(rememberedFormat(storageKey, formats));
	const current = $derived(formats.find((f) => f.id === format) ?? formats[0]);

	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	let box: HTMLDivElement | undefined = $state();
	const items = $derived<MenuItem[]>(
		formats.map((f) => ({
			label: f.label,
			icon: f.id === format ? Check : undefined,
			onselect: () => {
				format = f.id;
				rememberFormat(storageKey, f.id);
			},
		})),
	);
	function openMenu() {
		const r = box?.getBoundingClientRect();
		if (r) menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
</script>

<div class="export-split" class:sm={size === "sm"} bind:this={box}>
	<Button {variant} size={size} onclick={() => onexport(format)} title="{effectiveLabel} · {current.label}">
		{#snippet icon()}<Download />{/snippet}
		{effectiveLabel}{#if size === "sm"}<span class="fmt-inline">{current.label}</span>{/if}
	</Button>
	<IconButton label={$_("sharedControls.exportFormatAria", { values: { label: current.label } })} size={size === "sm" ? "sm" : undefined} variant="soft" aria-haspopup="menu" aria-expanded={!!menu} onclick={openMenu}><ChevronDown /></IconButton>
</div>
{#if hint && size !== "sm"}<p class="fmt">{current.label}{current.desc ? ` · ${current.desc}` : ""}</p>{/if}
<ContextMenu at={menu} label={$_("sharedControls.exportFormatMenuAria")} {items} onclose={() => (menu = null)} />

<style>
	.export-split {
		display: flex;
		gap: 2px;
		min-width: 0;
	}
	.export-split:not(.sm) > :global(:first-child) {
		flex: 1;
	}
	.export-split > :global(:first-child) {
		border-top-right-radius: 0;
		border-bottom-right-radius: 0;
	}
	.export-split > :global(:last-child) {
		border-top-left-radius: 0;
		border-bottom-left-radius: 0;
	}
	.fmt-inline {
		margin-left: 6px;
		font-size: var(--text-xs);
		opacity: 0.7;
	}
	.fmt {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
</style>
