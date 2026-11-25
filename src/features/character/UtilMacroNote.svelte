<script lang="ts">
	// 기능 스위치 밑 안내 (5-F-7-1, ADR-030 추가): 지금 프리셋에 그 매크로 자리가 없으면 지시문이 안 들어간다
	import { presetMacros, type UtilMacro } from "./presetMacros.svelte";

	let { presetId, macro }: { presetId: string | null; macro: UtilMacro } = $props();
</script>

{#if presetMacros.missing(presetId, macro)}
	<p class="note sc-util-macro-note" role="note">프리셋에 <code>{`{{${macro}}}`}</code>가 없어 작동하지 않을 수 있음</p>
{/if}

<style>
	.note {
		margin: 2px 0 0;
		color: var(--warning);
		font-size: var(--text-xs);
	}
	code {
		font-size: inherit;
	}
</style>
