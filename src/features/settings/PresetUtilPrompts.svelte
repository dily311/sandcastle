<script lang="ts">
	// 프리셋의 유틸 프롬프트 (5-F-7-3, ADR-030): 프리셋 > 전역(설정 > 앱 프롬프트) > 기본값.
	// 슬롯마다 접힌 칸. 비워 두면 전역을 따르고, 지금 쓰이는 글은 자리표시로 보인다
	import { onMount } from "svelte";
	import { Copy, X } from "@lucide/svelte";
	import type { AppPrompts_Serialize as AppPrompts, PromptSlot } from "$ipc/bindings";
	import { Badge, IconButton, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { current, ensureLoaded } from "./pluginSettingForm.svelte";

	/** 프리셋이 고친 글 (슬롯 이름 → 글). 빈 글은 저장하지 않음 */
	let { value = $bindable() }: { value: Record<string, string> } = $props();

	// 백엔드 PresetPrompts::SLOTS와 같은 것만 (번역·그림 프롬프트 쓰기는 프리셋 없이 돌아 전역만)
	const SLOTS: { key: keyof AppPrompts; label: string; vars: string }[] = [
		{ key: "chatImages", label: "캐릭터 에셋", vars: "{{img_keys}} · 자리 {{img_inprompt}}" },
		{ key: "autoImage", label: "이미지 자동 생성", vars: "자리 {{auto_image}}" },
		{ key: "vars", label: "AI 변수", vars: "{{var_list}} · 자리 {{vars}}" },
		{ key: "summary", label: "대화 요약", vars: "{max}" },
		{ key: "gameRules", label: "게임 규칙", vars: "한 줄에 규칙 하나" },
		{ key: "sceneRules", label: "씬 규칙", vars: "한 줄에 규칙 하나" },
	];

	let defaults = $state<AppPrompts | null>(null);
	onMount(async () => {
		ensureLoaded();
		defaults = (await call(commands.defaultAppPrompts(), { source: "prompts", silent: true }).catch(() => null)) as AppPrompts | null;
	});

	/** 프리셋이 비었을 때 쓰이는 글과 출처 */
	function fallback(key: keyof AppPrompts): { text: string; from: "전역" | "기본" } {
		const slot = (current()?.prompts as AppPrompts | undefined)?.[key] as PromptSlot | undefined;
		const chosen = slot?.items.find((v) => v.id === slot.selected)?.text.trim();
		if (chosen) return { text: chosen, from: "전역" };
		return { text: defaults?.[key]?.items[0]?.text ?? "", from: "기본" };
	}
	function set(key: string, text: string) {
		const next = { ...value };
		if (text.trim()) next[key] = text;
		else delete next[key];
		value = next;
	}
</script>

<div class="slots">
	{#each SLOTS as s (s.key)}
		{@const own = !!value[s.key]?.trim()}
		{@const fb = fallback(s.key)}
		<details class="slot sc-preset-util">
			<summary>
				<span class="nm">{s.label}</span>
				<Badge tone={own ? "accent" : "neutral"}>{own ? "프리셋" : fb.from}</Badge>
			</summary>
			<div class="body">
				<Textarea value={value[s.key] ?? ""} placeholder={fb.text} mono rows={4} autosize maxRows={16} aria-label="{s.label} 프롬프트" oninput={(e) => set(s.key, e.currentTarget.value)} />
				<div class="tools">
					<code>{s.vars}</code>
					<span class="sp"></span>
					{#if own}
						<IconButton label="비우기 ({fb.from} 값을 따름)" size="sm" onclick={() => set(s.key, "")}><X /></IconButton>
					{:else}
						<IconButton label="{fb.from} 값을 복사해 고치기" size="sm" disabled={!fb.text} onclick={() => set(s.key, fb.text)}><Copy /></IconButton>
					{/if}
				</div>
			</div>
		</details>
	{/each}
</div>

<style>
	.slots {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-3) 0;
	}
	.slot {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	summary {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		cursor: pointer;
		font-size: var(--text-sm);
	}
	.nm {
		flex: 1;
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: 0 var(--space-3) var(--space-2);
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.tools code {
		font-size: inherit;
	}
	.sp {
		flex: 1;
	}
</style>
