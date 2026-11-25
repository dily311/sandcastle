<script lang="ts">
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	// 메모리 확장: 대화 요약 (3-D, ADR-016). 켜는 것이 동의 — 오래된 대화가 쌓이면 자동으로 요약하고 요금이 듦
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Select } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
</script>

<Section title={$_("settings.summarySec.title")} lead={$_("settings.summarySec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.summarySec.api")}>
				<Select value={cur.summary.apiProfileId ?? ""} options={apiOptions($_("settings.summarySec.chatApi"))} onchange={(e) => save("summary", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.summary.apiProfileId}
				<Row label={$_("settings.summarySec.model")} desc={$_("settings.summarySec.modelDesc")}>
					<ModelPicker profile={library.apis.find((a) => a.id === cur.summary.apiProfileId)} value={cur.summary.model || null} onchange={(m) => save("summary", { model: m ?? "" })} label={$_("settings.summarySec.model")} />
				</Row>
			{/if}
		</Group>
		<Group title={$_("settings.summarySec.promptGroup")}>
			<PromptSlotEditor key="summary" desc={$_("settings.summarySec.promptDesc")} vars={"{max}"} />
		</Group>
	{/if}
</Section>
