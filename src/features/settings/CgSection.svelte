<script lang="ts">
	// 캐릭터 에셋(CG) 설정: 대화에서 <cg>키</cg>로 캐릭터 에셋을 보여주게 하는 지시
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import Row from "./Row.svelte";
	import { Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	appSetting.load();
</script>

<Section title="캐릭터 에셋" lead="캐릭터에 넣어 둔 이미지를 AI가 어울리는 순간에 <cg>키</cg>로 보여줌">
	<Group title="키">
		<Row label="같은 키 묶기" desc="smile.png·smile-1.png·smile-2.png를 키 smile 하나로. AI에게는 한 번만 알리고, 보일 때는 그중 하나를 메시지마다 무작위로">
			<Switch checked={!!appSetting.value?.cgGroupVariants} label="같은 키 묶기" hideLabel onchange={(v) => void appSetting.update({ cgGroupVariants: v })} />
		</Row>
	</Group>
	<Group title="CG 프롬프트">
		<PromptSlotEditor key="chatImages" desc="이미지 키가 있는 캐릭터와 대화할 때만 붙음" vars={"{{img_keys}} 이미지 키 목록 · {{char}} 캐릭터 · 프리셋 블록에 {{img_inprompt}}를 넣은 자리에 들어감 (없으면 안 들어감)"} />
	</Group>
</Section>
