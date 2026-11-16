<script lang="ts">
	// 게임 모드 설정: 게임 대화에 항상 앞에 붙는 규칙 (앱 프롬프트)
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
</script>

<Section title="게임 모드" lead="게임 대화(NPC·씬 입력)에 앱이 붙이는 규칙. 게임 데이터보다 항상 앞에 옴">
	<Group title="게임 대화 규칙">
		<PromptSlotEditor key="gameRules" desc="NPC 1:1 대화에 붙음. 한 줄에 규칙 하나. <updateRel>이 든 줄은 씬 입력에서 빠짐" />
	</Group>
	<Group title="씬 입력(서술자) 규칙">
		<PromptSlotEditor key="sceneRules" desc="씬 입력에 더해지는 규칙. 한 줄에 규칙 하나" />
	</Group>
</Section>
