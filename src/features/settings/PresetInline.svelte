<script lang="ts">
	// 프리셋을 그 자리에서 고치기 (이름·설명 빼고, 자동 저장). 대화·게임 패널의 컨텍스트 탭이 같이 씀
	import type { PromptBlocksInput_Deserialize as BlockInput, PromptPresetDetail_Serialize as PresetDetail } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import PresetEditor from "./PresetEditor.svelte";

	let { presetId, onsaved, model = null }: { presetId: string | null; onsaved?: () => void; model?: string | null } = $props();

	let detail = $state<PresetDetail | null>(null);
	let saving = $state(false);
	async function load(id: string | null) {
		if (!id) return void (detail = null);
		const d = await call(commands.getPromptPreset(id), { source: "preset", silent: true }).catch(() => null);
		if (presetId === id) detail = d;
	}
	$effect(() => {
		void load(presetId);
	});
	/** 편집기를 다시 그리지 않음: 고치던 자리를 잃지 않게 */
	async function save(preset: PresetDetail["preset"], blocks: BlockInput[]) {
		saving = true;
		try {
			const d = await call(commands.savePromptPreset({ ...preset, id: preset.id }, blocks), { source: "preset" });
			onsaved?.();
			return d;
		} finally {
			saving = false;
		}
	}
</script>

{#if detail}
	{#key detail.preset.id}
		<PresetEditor {detail} {saving} onsave={save} quick autosave {model} />
	{/key}
{:else if presetId}
	<p class="note">프리셋 불러오는 중…</p>
{:else}
	<p class="note">쓰는 프리셋 없음. 설정 &gt; 프리셋에서 만들 수 있음</p>
{/if}

<style>
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
