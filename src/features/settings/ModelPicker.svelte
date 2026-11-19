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
	import { apiLists, modelLabel } from "./providers";

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

	// 공급자 이름은 대소문자를 가리지 않고 맞춘다 (예전에 저장된 연결은 "OpenAI", 카탈로그는 "openai")
	const entry = $derived(apiLists.find((p) => p.provider.toLowerCase() === (profile?.provider ?? "").toLowerCase()));
	/** 목록에 보이는 이름: 사람이 읽는 이름이 있으면 "이름 (id)" */
	const nameOf = (id: string) => {
		const l = modelLabel(entry, id);
		return l === id ? id : `${l} (${id})`;
	};
	/** 목록에 보일 모델과 묶음 이름: 내장 목록은 카탈로그의 묶음, 그 뒤로 내가 추가한 모델 · 불러온 모델 */
	const choices = $derived.by(() => {
		if (!profile) return [];
		const seen = new Set<string>([profile.model]);
		const out: { id: string; group: string }[] = [];
		const add = (id: string | null | undefined, group: string) => {
			if (id && !seen.has(id)) {
				seen.add(id);
				out.push({ id, group });
			}
		};
		for (const g of entry?.modelList ?? []) for (const m of g.model) add(m.modelId, g.label);
		for (const m of profile.customModels ?? []) add(m, "내가 추가한 모델");
		for (const m of fetched) add(m, "공급자에서 불러온 모델");
		add(value, "내가 추가한 모델");
		return out;
	});
	const options = $derived([{ value: DEFAULT, label: `기본 모델 (${profile ? nameOf(profile.model) : "-"})` }, ...choices.map((m) => ({ value: m.id, label: nameOf(m.id), group: m.group }))]);

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
