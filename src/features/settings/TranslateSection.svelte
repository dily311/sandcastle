<script lang="ts">
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { library } from "$features/character/library.svelte";
	import { onMount } from "svelte";
	import { _, locale } from "svelte-i18n";
	import { defaultTargetLang } from "$lib/utils/i18n";
	import { Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { chatApiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
</script>

<Section title={$_("settings.translateSec.title")} lead={$_("settings.translateSec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.translateSec.api")} desc={$_("settings.translateSec.apiDesc")}>
				<Select value={cur.translate.apiProfileId ?? ""} options={chatApiOptions()} onchange={(e) => save("translate", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.translate.apiProfileId}
				<Row label={$_("settings.translateSec.model")} desc={$_("settings.translateSec.modelDesc")}>
					<ModelPicker profile={library.apis.find((a) => a.id === cur.translate.apiProfileId)} value={cur.translate.model || null} onchange={(m) => save("translate", { model: m ?? "" })} label={$_("settings.translateSec.model")} />
				</Row>
			{/if}
			<Row label={$_("settings.translateSec.targetLang")} desc={$_("settings.translateSec.targetLangDesc")}>
				<Input value={cur.translate.targetLang} placeholder={defaultTargetLang($locale ?? "en")} onchange={(e) => save("translate", { targetLang: e.currentTarget.value.trim() })} />
			</Row>
			<Row label={$_("settings.translateSec.inputLang")} desc={$_("settings.translateSec.inputLangDesc")}>
				<Input value={cur.translate.inputLang} onchange={(e) => save("translate", { inputLang: e.currentTarget.value.trim() || "English" })} />
			</Row>
			<Row label={$_("settings.translateSec.autoTranslate")} desc={$_("settings.translateSec.autoTranslateDesc")}>
				<Switch checked={cur.translate.autoTranslate} label={$_("settings.translateSec.autoTranslate")} hideLabel onchange={(v) => save("translate", { autoTranslate: v })} />
			</Row>
		</Group>
		<Group title={$_("settings.translateSec.promptGroup")}>
			<PromptSlotEditor key="translate" desc={$_("settings.translateSec.promptDesc")} vars={"{{lang}}"} />
		</Group>
	{/if}
</Section>
