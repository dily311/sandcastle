<script lang="ts">
	// 캐릭터 에셋(CG) 설정: 대화에서 <cg>키</cg>로 캐릭터 에셋을 보여주게 하는 지시
	import { _ } from "svelte-i18n";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import Row from "./Row.svelte";
	import { Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	appSetting.load();
</script>

<Section title={$_("settings.cgSec.title")} lead={$_("settings.cgSec.lead")}>
	<Group title={$_("settings.cgSec.keyGroup")}>
		<Row label={$_("settings.cgSec.groupVariants")} desc={$_("settings.cgSec.groupVariantsDesc")}>
			<Switch checked={!!appSetting.value?.cgGroupVariants} label={$_("settings.cgSec.groupVariants")} hideLabel onchange={(v) => void appSetting.update({ cgGroupVariants: v })} />
		</Row>
	</Group>
	<Group title={$_("settings.cgSec.promptGroup")}>
		<PromptSlotEditor key="chatImages" desc={$_("settings.cgSec.promptDesc")} vars={"{{img_keys}} · {{char}} · {{img_inprompt}}"} />
	</Group>
</Section>
