<script lang="ts">
	// 캐릭터 이미지(CG) 설정: 대화에서 <cg>키</cg>로 캐릭터 이미지를 보여주게 하는 지시
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
</script>

<Section title="캐릭터 이미지" lead="캐릭터에 넣어 둔 이미지를 AI가 어울리는 순간에 <cg>키</cg>로 보여줌">
	<Group title="CG 프롬프트">
		<PromptSlotEditor key="chatImages" desc="이미지 키가 있는 캐릭터와 대화할 때만 붙음" vars={"{{keys}} 이미지 키 목록 · {{char}} 캐릭터"} />
	</Group>
</Section>
