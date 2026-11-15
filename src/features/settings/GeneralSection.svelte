<script lang="ts">
	import { onMount } from "svelte";
	import { Select, Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => appSetting.load());
	const s = $derived(appSetting.value);
</script>

<Section title="기본 설정" lead="앱 전체에 걸리는 설정. 대화·폰 설정은 각 탭에 있음">
	{#if s}
		<Group>
			<Row label="언어" desc="다른 언어는 준비 중">
				<Select value="ko" disabled options={[{ value: "ko", label: "한국어" }]} aria-label="언어" />
			</Row>
			<Row label="자동 업데이트" desc="새 버전이 나오면 받아서 설치">
				<Switch checked={s.autoUpdate} label="자동 업데이트" hideLabel onchange={(v) => appSetting.update({ autoUpdate: v })} />
			</Row>
		</Group>
	{/if}
</Section>
