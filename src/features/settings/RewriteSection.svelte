<script lang="ts">
	// 자동 재생성 (3-B-4, ADR-017): linter가 찾은 문제로 응답을 새 스와이프로 다시 만든다. 켜면 걸릴 때마다 응답 하나의 요금이 더 든다
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Input, Switch, TagInput } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const clamp = (v: string) => Math.max(1, Math.min(3, Math.floor(Number(v)) || 1));
	// 금지어 목록은 메시지 linter와 따로 (이 목록의 말이 응답에 나오면 다시 만듦). 태그가 바뀌면 저장한다
	let bannedWords = $state<string[]>([]);
	$effect(() => {
		bannedWords = [...(cur?.rewrite.bannedWords ?? [])];
	});
	$effect(() => {
		const next = [...new Set(bannedWords.map((w) => w.trim()).filter(Boolean))].slice(0, 100);
		if (cur && JSON.stringify(next) !== JSON.stringify(cur.rewrite.bannedWords)) void save("rewrite", { bannedWords: next });
	});
</script>

<Section title={$_("settings.rewriteSec.title")} lead={$_("settings.rewriteSec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.rewriteSec.maxAttempts")}>
				<Input type="number" min="1" max="3" value={String(cur.rewrite.maxAttempts)} onchange={(e) => save("rewrite", { maxAttempts: clamp(e.currentTarget.value) })} />
			</Row>
			<Row label={$_("settings.rewriteSec.banned")} desc={$_("settings.rewriteSec.bannedDesc")}>
				<Switch checked={cur.rewrite.banned} label={$_("settings.rewriteSec.banned")} hideLabel onchange={(v) => save("rewrite", { banned: v })} />
			</Row>
			{#if cur.rewrite.banned}
				<Row label={$_("settings.rewriteSec.bannedList")} desc={$_("settings.rewriteSec.bannedListDesc")} stacked>
					<TagInput bind:value={bannedWords} placeholder={$_("settings.rewriteSec.bannedPlaceholder")} label={$_("settings.rewriteSec.bannedList")} />
				</Row>
			{/if}
			<Row label={$_("settings.rewriteSec.stuck")}>
				<Switch checked={cur.rewrite.stuck} label={$_("settings.rewriteSec.stuck")} hideLabel onchange={(v) => save("rewrite", { stuck: v })} />
			</Row>
		</Group>
	{/if}
</Section>
