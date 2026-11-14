<script lang="ts">
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	// 지금 앱에서 실제로 동작하는 키만 보여줌
	const groups = $derived([
		{
			title: "이동",
			items: [
				{ label: "뒤로 / 앞으로", keys: [["Alt", "←"], ["Alt", "→"]] },
				{ label: "뒤로 / 앞으로 (마우스)", keys: [["마우스 4"], ["마우스 5"]] },
			],
		},
		{
			title: "대화",
			items: [
				{ label: "보내기", keys: [appSetting.sendOnEnter ? ["Enter"] : ["Ctrl", "Enter"]] },
				{ label: "줄바꿈", keys: [["Shift", "Enter"]] },
				{ label: "편집 취소 / 메뉴 닫기", keys: [["Esc"]] },
			],
		},
	]);
</script>

<Section title="단축키" lead="지금 쓸 수 있는 키 목록. 키 바꾸기는 준비 중">
	{#each groups as g (g.title)}
		<Group title={g.title}>
			{#each g.items as it (it.label)}
				<Row label={it.label}>
					<span class="keys">
						{#each it.keys as combo, i (i)}
							{#if i > 0}<span class="or">/</span>{/if}
							{#each combo as k (k)}<kbd>{k}</kbd>{/each}
						{/each}
					</span>
				</Row>
			{/each}
		</Group>
	{/each}
</Section>

<style>
	.keys {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.or {
		margin: 0 4px;
		color: var(--text-faint);
	}
	kbd {
		min-width: 24px;
		padding: 2px 6px;
		border: 1px solid var(--border-strong);
		border-bottom-width: 2px;
		border-radius: 5px;
		background: var(--surface);
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		font-weight: 600;
		text-align: center;
	}
</style>
