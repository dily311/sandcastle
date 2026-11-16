<script lang="ts">
	import { onMount } from "svelte";
	import { Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
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
	{/if}
</Section>
