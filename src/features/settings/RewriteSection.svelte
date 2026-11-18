<script lang="ts">
	// 자동 재생성 (3-B-4, ADR-017): linter가 찾은 문제로 응답을 새 스와이프로 다시 만든다. 켜면 걸릴 때마다 응답 하나의 요금이 더 든다
	import { onMount } from "svelte";
	import { Input, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const clamp = (v: string) => Math.max(1, Math.min(3, Math.floor(Number(v)) || 1));
</script>

<Section title="자동 재생성" lead="응답에서 금지어나 단어 반복 오류가 발견되면 새 스와이프로 다시 만듦 · 응답 하나당 최대 시도 횟수만큼 요금이 더 듦">
	{#if cur}
		<Group>
			<Row label="최대 시도">
				<Input type="number" min="1" max="3" value={String(cur.rewrite.maxAttempts)} onchange={(e) => save("rewrite", { maxAttempts: clamp(e.currentTarget.value) })} />
			</Row>
			<Row label="금지어">
				<Switch checked={cur.rewrite.banned} label="금지어" hideLabel onchange={(v) => save("rewrite", { banned: v })} />
			</Row>
			<Row label="단어 반복 오류">
				<Switch checked={cur.rewrite.stuck} label="단어 반복 오류" hideLabel onchange={(v) => save("rewrite", { stuck: v })} />
			</Row>
		</Group>
	{/if}
</Section>
