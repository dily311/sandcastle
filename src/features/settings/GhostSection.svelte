<script lang="ts">
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	// 대필 (5-D, ADR-018): 입력칸의 펜 버튼으로 AI가 내 말투의 다음 대사를 초안으로 써 줌. 누를 때마다 응답 하나의 요금
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const LENGTHS = $derived([
		{ value: "short", label: $_("settings.ghostSec.short") },
		{ value: "medium", label: $_("settings.ghostSec.medium") },
		{ value: "long", label: $_("settings.ghostSec.long") },
	]);
</script>

<Section title={$_("settings.ghostSec.title")} lead={$_("settings.ghostSec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.ghostSec.api")}>
				<Select value={cur.ghost.apiProfileId ?? ""} options={apiOptions($_("settings.summarySec.chatApi"))} onchange={(e) => save("ghost", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.ghost.apiProfileId}
				<Row label={$_("settings.ghostSec.model")} desc={$_("settings.ghostSec.modelDesc")}>
					<ModelPicker profile={library.apis.find((a) => a.id === cur.ghost.apiProfileId)} value={cur.ghost.model || null} onchange={(m) => save("ghost", { model: m ?? "" })} label={$_("settings.ghostSec.model")} />
				</Row>
			{/if}
			<Row label={$_("settings.ghostSec.length")}>
				<Select value={cur.ghost.length} options={LENGTHS} onchange={(e) => save("ghost", { length: e.currentTarget.value })} />
			</Row>
			<Row label={$_("settings.ghostSec.guide")}>
				<Input value={cur.ghost.guide} maxlength={300} placeholder={$_("settings.ghostSec.guidePlaceholder")} onchange={(e) => save("ghost", { guide: e.currentTarget.value.trim() })} />
			</Row>
			<Row label={$_("settings.ghostSec.sendWithoutDraft")}>
				<Switch checked={cur.ghost.sendWithoutDraft} label={$_("settings.ghostSec.sendWithoutDraft")} hideLabel onchange={(v) => save("ghost", { sendWithoutDraft: v })} />
			</Row>
		</Group>
		<Group title={$_("settings.ghostSec.promptGroup")}>
			<PromptSlotEditor key="ghost" desc={$_("settings.ghostSec.promptDesc")} vars={"{{user}} · {{char}} · {{length}}"} />
		</Group>
	{/if}
</Section>
