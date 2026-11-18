<script lang="ts">
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { library } from "$features/character/library.svelte";
	import { onMount } from "svelte";
	import { Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { chatApiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
</script>

<Section title="번역" lead="AI 답변을 내 언어로, 내 입력을 다른 언어로. 연결한 채팅 AI로 번역함">
	{#if cur}
		<Group>
			<Row label="번역에 쓸 API" desc="싸고 빠른 모델이면 충분">
				<Select value={cur.translate.apiProfileId ?? ""} options={chatApiOptions()} onchange={(e) => save("translate", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.translate.apiProfileId}
				<Row label="번역 모델" desc="비우면 그 API의 기본 모델">
					<ModelPicker profile={library.apis.find((a) => a.id === cur.translate.apiProfileId)} value={cur.translate.model || null} onchange={(m) => save("translate", { model: m ?? "" })} label="번역 모델" />
				</Row>
			{/if}
			<Row label="AI 답변을 이 언어로">
				<Input value={cur.translate.targetLang} onchange={(e) => save("translate", { targetLang: e.currentTarget.value.trim() || "한국어" })} />
			</Row>
			<Row label="내 입력을 이 언어로" desc="입력창 + 메뉴의 '입력을 번역해서 바꾸기'">
				<Input value={cur.translate.inputLang} onchange={(e) => save("translate", { inputLang: e.currentTarget.value.trim() || "English" })} />
			</Row>
			<Row label="새 답변 자동 번역" desc="답변이 오면 바로 번역해서 보여줌 (원문 보기로 되돌림)">
				<Switch checked={cur.translate.autoTranslate} label="자동 번역" hideLabel onchange={(v) => save("translate", { autoTranslate: v })} />
			</Row>
		</Group>
		<Group title="번역 프롬프트">
			<PromptSlotEditor key="translate" desc="AI 답변·입력·게임 대사를 옮길 때 보내는 지시" vars={"{{lang}} 옮길 언어"} />
		</Group>
	{/if}
</Section>
