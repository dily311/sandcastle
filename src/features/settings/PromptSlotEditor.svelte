<script lang="ts">
	// 앱 프롬프트 자리 하나 (번역·그림 프롬프트·CG·게임 규칙 등). 기본값 + 사용자가 만든 프롬프트들 중 하나를 고른다.
	// 기본값은 고칠 수 없고, "새로 추가"로 지금 글을 복제해 고친다. 빈 프롬프트는 기본값으로 돌아간다
	import { onMount } from "svelte";
	import { Copy, Pencil, Plus, Trash2 } from "@lucide/svelte";
	import type { AppPrompts_Serialize as AppPrompts, PromptSlot, PromptVariant } from "$ipc/bindings";
	import { Button, Select, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	type Key = keyof AppPrompts;
	interface Props {
		key: Key;
		desc: string;
		/** 넣을 수 있는 값 안내 (예: "{{lang}} 옮길 언어") */
		vars?: string;
	}
	let { key, desc, vars }: Props = $props();

	let defaults = $state<AppPrompts | null>(null);
	let loadingDefaults: Promise<AppPrompts | null> | null = null;
	onMount(async () => {
		ensureLoaded();
		loadingDefaults ??= call(commands.defaultAppPrompts(), { source: "prompts", silent: true }).catch(() => null) as Promise<AppPrompts | null>;
		defaults = await loadingDefaults;
	});

	const slot = $derived<PromptSlot>(((current()?.prompts as AppPrompts | undefined)?.[key] as PromptSlot | undefined) ?? { selected: "", items: [] });
	const defaultText = $derived(defaults?.[key]?.items[0]?.text ?? "");
	const chosen = $derived<PromptVariant | undefined>(slot.items.find((v) => v.id === slot.selected));
	const options = $derived([{ value: "", label: "기본값" }, ...slot.items.map((v) => ({ value: v.id, label: v.name || "이름 없음" }))]);

	let draft = $state<string | null>(null);
	$effect(() => {
		void slot.selected;
		draft = null;
	});
	const shown = $derived(chosen ? (draft ?? chosen.text) : defaultText);

	function put(next: PromptSlot) {
		save("prompts", { [key]: next } as Partial<AppPrompts>);
	}
	function pick(id: string) {
		put({ ...slot, selected: id });
	}
	async function add() {
		const name = await promptText({ title: "새 프롬프트", message: "지금 보이는 글을 복제해 새 프롬프트를 만듦", input: { value: `내 프롬프트 ${slot.items.length + 1}`, placeholder: "이름" }, confirmLabel: "만들기" });
		if (!name) return;
		const v: PromptVariant = { id: crypto.randomUUID(), name: name.trim(), text: shown };
		put({ selected: v.id, items: [...slot.items, v] });
	}
	async function rename() {
		if (!chosen) return;
		const name = await promptText({ title: "이름 바꾸기", message: "", input: { value: chosen.name, placeholder: "이름" }, confirmLabel: "바꾸기" });
		if (!name) return;
		put({ ...slot, items: slot.items.map((v) => (v.id === chosen.id ? { ...v, name: name.trim() } : v)) });
	}
	async function remove() {
		if (!chosen) return;
		if (!(await confirmAction({ title: "프롬프트 삭제", message: `'${chosen.name}'을(를) 지움. 기본값으로 돌아감`, confirmLabel: "삭제", danger: true }))) return;
		put({ selected: "", items: slot.items.filter((v) => v.id !== chosen.id) });
	}
	function commit() {
		if (!chosen || draft === null || draft === chosen.text) return;
		put({ ...slot, items: slot.items.map((v) => (v.id === chosen.id ? { ...v, text: draft ?? "" } : v)) });
		draft = null;
		toasts.success("프롬프트 저장함");
	}
</script>

<div class="slot">
	<p class="desc">{desc}{#if vars} · 넣을 수 있는 값: <code>{vars}</code>{/if}</p>
	<div class="bar">
		<Select value={slot.selected && chosen ? slot.selected : ""} {options} aria-label="쓸 프롬프트" onchange={(e) => pick(e.currentTarget.value)} />
		<Button size="sm" variant="ghost" onclick={add}>{#snippet icon()}{#if chosen}<Copy />{:else}<Plus />{/if}{/snippet}{chosen ? "복제" : "새로 추가"}</Button>
		{#if chosen}
			<Button size="sm" variant="ghost" onclick={rename}>{#snippet icon()}<Pencil />{/snippet}이름</Button>
			<Button size="sm" variant="ghost" onclick={remove}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
		{/if}
	</div>
	<Textarea value={shown} readonly={!chosen} autosize rows={4} maxRows={16} mono aria-label="프롬프트 내용" oninput={(e) => (draft = e.currentTarget.value)} onblur={commit} />
	{#if !chosen}<p class="hint">기본값은 고칠 수 없음 · "새로 추가"로 복제해서 고침</p>{:else if draft !== null && draft !== chosen.text}<p class="hint">칸을 벗어나면 저장됨</p>{/if}
</div>

<style>
	.slot {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4) 0;
	}
	.desc,
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.hint {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.desc code {
		font-size: var(--text-xs);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.bar :global(.select) {
		min-width: 180px;
	}
</style>
