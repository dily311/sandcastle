<script lang="ts">
	// 게임 모드 설정: 게임 대화에 항상 앞에 붙는 규칙 (앱 프롬프트)
	import { _ } from "svelte-i18n";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
</script>

<Section title={$_("settings.gameSec.title")} lead={$_("settings.gameSec.lead")}>
	<Group title={$_("settings.gameSec.gameRules")}>
		<PromptSlotEditor key="gameRules" desc={$_("settings.gameSec.gameRulesDesc")} />
	</Group>
	<Group title={$_("settings.gameSec.sceneRules")}>
		<PromptSlotEditor key="sceneRules" desc={$_("settings.gameSec.sceneRulesDesc")} />
	</Group>
</Section>
