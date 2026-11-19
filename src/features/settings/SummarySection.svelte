<script lang="ts">
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	// 메모리 확장: 대화 요약 (3-D, ADR-016). 켜는 것이 동의 — 오래된 대화가 쌓이면 자동으로 요약하고 요금이 듦
	import { onMount } from "svelte";
	import { Select } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
</script>

<Section title="대화 요약" lead="프롬프트 창 밖으로 밀려난 대화가 쌓이면 자동으로 줄거리를 요약해 AI에게 전달 · 요금이 듦">
	{#if cur}
		<Group>
			<Row label="요약 API">
				<Select value={cur.summary.apiProfileId ?? ""} options={apiOptions("대화의 API")} onchange={(e) => save("summary", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.summary.apiProfileId}
				<Row label="요약 모델" desc="비우면 그 API의 기본 모델">
					<ModelPicker profile={library.apis.find((a) => a.id === cur.summary.apiProfileId)} value={cur.summary.model || null} onchange={(m) => save("summary", { model: m ?? "" })} label="요약 모델" />
				</Row>
			{/if}
		</Group>
	{/if}
</Section>
