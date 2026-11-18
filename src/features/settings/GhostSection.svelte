<script lang="ts">
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	// 대필 (5-D, ADR-018): 입력칸의 펜 버튼으로 AI가 내 말투의 다음 대사를 초안으로 써 줌. 누를 때마다 응답 하나의 요금
	import { onMount } from "svelte";
	import { Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const LENGTHS = [
		{ value: "short", label: "짧게" },
		{ value: "medium", label: "보통" },
		{ value: "long", label: "길게" },
	];
</script>

<Section title="대필" lead="초안을 AI가 내 말투의 다음 대사로 써서 입력칸에 채움 · 보내지는 않음 · 누를 때마다 응답 하나의 요금">
	{#if cur}
		<Group>
			<Row label="대필 API">
				<Select value={cur.ghost.apiProfileId ?? ""} options={apiOptions("대화의 API")} onchange={(e) => save("ghost", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.ghost.apiProfileId}
				<Row label="대필 모델" desc="비우면 그 API의 기본 모델">
					<ModelPicker profile={library.apis.find((a) => a.id === cur.ghost.apiProfileId)} value={cur.ghost.model || null} onchange={(m) => save("ghost", { model: m ?? "" })} label="대필 모델" />
				</Row>
			{/if}
			<Row label="길이">
				<Select value={cur.ghost.length} options={LENGTHS} onchange={(e) => save("ghost", { length: e.currentTarget.value })} />
			</Row>
			<Row label="추가 지침">
				<Input value={cur.ghost.guide} maxlength={300} placeholder="예) 1인칭, 대사 위주" onchange={(e) => save("ghost", { guide: e.currentTarget.value.trim() })} />
			</Row>
			<Row label="초안 없이 바로 요청">
				<Switch checked={cur.ghost.sendWithoutDraft} label="초안 없이 바로 요청" hideLabel onchange={(v) => save("ghost", { sendWithoutDraft: v })} />
			</Row>
		</Group>
	{/if}
</Section>
