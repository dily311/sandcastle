<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Select, Textarea } from "$shared/ui";
	import { library } from "$features/character/library.svelte";
	import { chatApiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	let phonePrompt = $state("");

	onMount(async () => {
		ensureLoaded();
		await appSetting.load();
		phonePrompt = appSetting.value?.phoneModePrompt ?? "";
	});

	const s = $derived(appSetting.value);
	const plug = $derived(current());
	const presetOptions = $derived([{ value: "", label: "앱 기본 프리셋" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	async function savePhone() {
		await appSetting.update({ phoneModePrompt: phonePrompt });
		toasts.success("저장함");
	}
</script>

<Section title="폰" lead="폰 화면의 메신저 대화에 쓰는 설정">
	{#if plug}
		<Group title="기본 API·프리셋" >
			<Row label="API" desc="폰 대화에 따로 고르지 않았을 때 (대화 설정에서 고른 것이 먼저)">
				<Select value={plug.phone.apiProfileId ?? ""} options={chatApiOptions("앱 기본 API")} onchange={(e) => save("phone", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label="프리셋" desc="짧은 메신저 답장에 맞춘 프리셋을 따로 두면 좋음">
				<Select value={plug.phone.presetId ?? ""} options={presetOptions} onchange={(e) => save("phone", { presetId: orNull(e.currentTarget.value) })} />
			</Row>
		</Group>
	{/if}
	{#if s}
		<Group title="프롬프트">
			<Row label="폰 모드 프롬프트" desc={"폰 대화에만 붙는 시스템 프롬프트. {{char}}, {{user}} 사용 가능. 비우면 붙이지 않음"} stacked>
				<Textarea bind:value={phonePrompt} autosize maxRows={14} rows={5} mono aria-label="폰 모드 프롬프트" />
				<div class="end">
					<Button size="sm" variant="ghost" disabled={phonePrompt === s.phoneModePrompt} onclick={() => (phonePrompt = s.phoneModePrompt)}>되돌리기</Button>
					<Button size="sm" variant="primary" disabled={phonePrompt === s.phoneModePrompt} onclick={savePhone}>저장</Button>
				</div>
			</Row>
		</Group>
	{/if}
</Section>

<style>
	.end {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
</style>
