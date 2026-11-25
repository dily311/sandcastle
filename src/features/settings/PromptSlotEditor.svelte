<script lang="ts">
	// 앱 프롬프트 자리 하나 (번역·그림 프롬프트·CG·게임 규칙 등). 기본값 + 사용자가 만든 프롬프트들 중 하나를 고른다.
	// 기본값은 고칠 수 없고, "새로 추가"로 지금 글을 복제해 고친다. 빈 프롬프트는 기본값으로 돌아간다
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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
	const options = $derived([{ value: "", label: $_("settings.promptSlot.default") }, ...slot.items.map((v) => ({ value: v.id, label: v.name || $_("settings.promptSlot.unnamed") }))]);

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
		const name = await promptText({
			title: $_("settings.promptSlot.newTitle"),
			message: $_("settings.promptSlot.newMsg"),
			input: { value: $_("settings.promptSlot.newDefaultName", { values: { n: slot.items.length + 1 } }), placeholder: $_("settings.promptSlot.namePlaceholder") },
			confirmLabel: $_("settings.promptSlot.createBtn"),
		});
		if (!name) return;
		const v: PromptVariant = { id: crypto.randomUUID(), name: name.trim(), text: shown };
		put({ selected: v.id, items: [...slot.items, v] });
	}
	async function rename() {
		if (!chosen) return;
		const name = await promptText({
			title: $_("settings.promptSlot.renameTitle"),
			message: "",
			input: { value: chosen.name, placeholder: $_("settings.promptSlot.namePlaceholder") },
			confirmLabel: $_("settings.promptSlot.renameBtn"),
		});
		if (!name) return;
		put({ ...slot, items: slot.items.map((v) => (v.id === chosen.id ? { ...v, name: name.trim() } : v)) });
	}
	async function remove() {
		if (!chosen) return;
		if (!(await confirmAction({ title: $_("settings.promptSlot.deleteTitle"), message: $_("settings.promptSlot.deleteMsg", { values: { name: chosen.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		put({ selected: "", items: slot.items.filter((v) => v.id !== chosen.id) });
	}
	function commit() {
		if (!chosen || draft === null || draft === chosen.text) return;
		put({ ...slot, items: slot.items.map((v) => (v.id === chosen.id ? { ...v, text: draft ?? "" } : v)) });
		draft = null;
		toasts.success($_("settings.promptSlot.savedToast"));
	}
</script>

<div class="slot">
	<p class="desc">{desc}{#if vars} · {$_("settings.promptSlot.varsPrefix")} <code>{vars}</code>{/if}</p>
	<div class="bar">
		<Select value={slot.selected && chosen ? slot.selected : ""} {options} aria-label={$_("settings.promptSlot.selectAria")} onchange={(e) => pick(e.currentTarget.value)} />
		<Button size="sm" variant="ghost" onclick={add}>{#snippet icon()}{#if chosen}<Copy />{:else}<Plus />{/if}{/snippet}{chosen ? $_("settings.promptSlot.duplicate") : $_("settings.promptSlot.addNew")}</Button>
		{#if chosen}
			<Button size="sm" variant="ghost" onclick={rename}>{#snippet icon()}<Pencil />{/snippet}{$_("settings.promptSlot.renameShort")}</Button>
			<Button size="sm" variant="ghost" onclick={remove}>{#snippet icon()}<Trash2 />{/snippet}{$_("common.delete")}</Button>
		{/if}
	</div>
	<Textarea value={shown} readonly={!chosen} autosize rows={4} maxRows={16} mono aria-label={$_("settings.promptSlot.contentAria")} oninput={(e) => (draft = e.currentTarget.value)} onblur={commit} />
	{#if !chosen}<p class="hint">{$_("settings.promptSlot.readonlyHint")}</p>{:else if draft !== null && draft !== chosen.text}<p class="hint">{$_("settings.promptSlot.blurSaveHint")}</p>{/if}
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
