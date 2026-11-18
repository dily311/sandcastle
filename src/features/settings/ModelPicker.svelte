<script lang="ts">
	// 모델 고르기 (ADR-020): 대화·기능이 API 프로필의 기본 모델 대신 쓸 모델.
	// 목록 = 프로필의 기본 모델 → 앱에 내장된 이 공급자의 모델 → 내가 추가한 모델 → (요청하면) 공급자에서 불러온 모델.
	// 목록에 없으면 "직접 입력…" — 입력한 이름은 이 프로필의 "내가 추가한 모델"로 남는다.
	import { Ellipsis } from "@lucide/svelte";
	import type { ApiProfiles } from "$ipc/bindings";
	import { ContextMenu, IconButton, Select, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import { apiLists } from "./providers";

	interface Props {
		/** 모델을 고를 API 프로필 */
		profile: ApiProfiles | undefined;
		/** 고른 모델. null이면 프로필의 기본 모델을 따라감 */
		value: string | null | undefined;
		onchange: (model: string | null) => void;
		label?: string;
	}
	let { profile, value, onchange, label = "모델" }: Props = $props();
	const src = { source: "model" };

	const DEFAULT = "";
	let fetched = $state<string[]>([]);
	// 프로필이 바뀌면 불러온 목록은 버림
	$effect(() => {
		void profile?.id;
		fetched = [];
	});

	const builtin = $derived(apiLists.find((p) => p.provider === profile?.provider)?.modelList.flatMap((g) => g.model) ?? []);
	const choices = $derived.by(() => {
		if (!profile) return [];
		const seen = new Set<string>([profile.model]);
		const out: string[] = [];
		for (const m of [...builtin, ...(profile.customModels ?? []), ...fetched, ...(value ? [value] : [])]) {
			if (m && !seen.has(m)) {
				seen.add(m);
				out.push(m);
			}
		}
		return out;
	});
	const options = $derived([{ value: DEFAULT, label: `기본 모델 (${profile?.model ?? "-"})` }, ...choices.map((m) => ({ value: m, label: m }))]);

	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const items = $derived<MenuItem[]>([
		{ label: "직접 입력…", onselect: typeModel },
		{ label: "공급자에서 목록 불러오기", disabled: !profile, onselect: fetchModels },
	]);
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}

	async function typeModel() {
		if (!profile) return;
		const t = await promptText({ title: "모델 직접 입력", message: "이 API의 \"내가 추가한 모델\"로 남음", input: { value: "", placeholder: "모델 이름" }, confirmLabel: "고르기" });
		const name = t?.trim();
		if (!name) return;
		const list = await call(commands.addApiProfileModel(profile.id, name), src).catch(() => null);
		if (list) {
			const p = library.apis.find((a) => a.id === profile.id);
			if (p) p.customModels = list;
		}
		onchange(name === profile.model ? null : name);
	}
	async function fetchModels() {
		if (!profile) return;
		try {
			fetched = await call(commands.getModels(profile.id), src);
			toasts.success(`모델 ${fetched.length}개 불러옴`);
		} catch {
			/* 토스트는 call이 띄움 */
		}
	}
</script>

<div class="picker">
	<Select value={value ?? DEFAULT} {options} aria-label={label} disabled={!profile} onchange={(e) => onchange(e.currentTarget.value || null)} />
	<IconButton label="{label} 더 보기 (직접 입력·목록 불러오기)" size="sm" disabled={!profile} onclick={openMenu}><Ellipsis /></IconButton>
</div>
<ContextMenu at={menu} {items} label="모델 메뉴" onclose={() => (menu = null)} />

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
	}
	.picker :global(select) {
		min-width: 0;
	}
</style>
