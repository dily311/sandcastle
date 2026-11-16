<script lang="ts">
	// 앱 프롬프트: 앱이 AI에 보내는 고정 지시문을 고친다. 빈 칸 = 기본값 (언제든 되돌릴 수 있음)
	import { onMount } from "svelte";
	import { RotateCcw } from "@lucide/svelte";
	import type { AppPrompts } from "$ipc/bindings";
	import { Badge, Button, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	type Key = keyof AppPrompts;
	const ITEMS: { key: Key; title: string; desc: string; vars?: string }[] = [
		{ key: "translate", title: "번역", desc: "AI 답변·입력·게임 대사를 옮길 때 (번역 플러그인)", vars: "{{lang}} 옮길 언어" },
		{ key: "imagePrompt", title: "장면 → 그림 프롬프트", desc: "이미지 생성에서 AI가 장면을 영어 프롬프트로 정리할 때" },
		{ key: "imagePromptNai", title: "장면 → NovelAI 태그", desc: "이미지 API가 NovelAI일 때 태그로 정리하는 지시" },
		{ key: "chatImages", title: "캐릭터 이미지(CG) 규칙", desc: "대화에서 <cg>키</cg>로 캐릭터 이미지를 보여주게 하는 지시", vars: "{{keys}} 이미지 키 목록 · {{char}} 캐릭터" },
		{ key: "gameRules", title: "게임 대화 규칙", desc: "게임 NPC 대화에 항상 앞에 붙는 규칙. 한 줄에 하나. <updateRel>이 든 줄은 씬 입력에서 빠짐" },
		{ key: "sceneRules", title: "씬 입력(서술자) 규칙", desc: "게임 씬 입력에 더해지는 규칙. 한 줄에 하나" },
	];

	let defaults = $state<AppPrompts | null>(null);
	let drafts = $state<Partial<Record<Key, string>>>({});
	onMount(async () => {
		ensureLoaded();
		defaults = await call(commands.defaultAppPrompts(), { source: "prompts", silent: true }).catch(() => null);
	});
	const cur = $derived(current());
	/** 사용자가 고친 값 (빈 칸이면 기본값을 쓰는 중) */
	const saved = (k: Key) => (cur?.prompts?.[k] ?? "").trim();
	const shown = (k: Key) => drafts[k] ?? (saved(k) || defaults?.[k] || "");
	const dirty = (k: Key) => drafts[k] !== undefined && drafts[k] !== (saved(k) || defaults?.[k] || "");

	function persist(k: Key, value: string) {
		const v = value.trim() === (defaults?.[k] ?? "").trim() ? "" : value;
		save("prompts", { [k]: v });
		const { [k]: _drop, ...rest } = drafts;
		drafts = rest;
		toasts.success(v ? "저장함" : "기본값으로 저장함");
	}
	function reset(k: Key) {
		save("prompts", { [k]: "" });
		const { [k]: _drop, ...rest } = drafts;
		drafts = rest;
		toasts.success("기본값으로 되돌림");
	}
</script>

<Section title="앱 프롬프트" lead="앱이 AI에 보내는 지시문. 고치면 다음 호출부터 쓰고, 기본값으로 언제든 되돌릴 수 있음">
	{#if cur && defaults}
		{#each ITEMS as it (it.key)}
			<Group title={it.title}>
				{#snippet aside()}
					<div class="aside">
						{#if saved(it.key)}<Badge tone="accent">고침</Badge>{:else}<Badge>기본값</Badge>{/if}
						{#if saved(it.key)}<Button size="sm" variant="ghost" onclick={() => reset(it.key)}>{#snippet icon()}<RotateCcw />{/snippet}기본값으로</Button>{/if}
					</div>
				{/snippet}
				<div class="item">
					<p class="desc">{it.desc}{#if it.vars} · 넣을 수 있는 값: <code>{it.vars}</code>{/if}</p>
					<Textarea value={shown(it.key)} autosize rows={4} maxRows={18} mono aria-label="{it.title} 프롬프트" oninput={(e) => (drafts = { ...drafts, [it.key]: e.currentTarget.value })} />
					{#if dirty(it.key)}
						<div class="btns">
							<Button size="sm" variant="ghost" onclick={() => { const { [it.key]: _d, ...rest } = drafts; drafts = rest; }}>취소</Button>
							<Button size="sm" variant="primary" onclick={() => persist(it.key, drafts[it.key] ?? "")}>저장</Button>
						</div>
					{/if}
				</div>
			</Group>
		{/each}
	{/if}
</Section>

<style>
	.aside {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.item {
		padding: var(--space-4) 0;
	}
	.desc {
		margin-bottom: var(--space-2);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.desc code {
		font-size: var(--text-xs);
	}
	.btns {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
</style>
