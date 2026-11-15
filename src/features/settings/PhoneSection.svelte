<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Textarea } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	let phonePrompt = $state("");

	onMount(async () => {
		await appSetting.load();
		phonePrompt = appSetting.value?.phoneModePrompt ?? "";
	});

	const s = $derived(appSetting.value);
	async function savePhone() {
		await appSetting.update({ phoneModePrompt: phonePrompt });
		toasts.success("저장함");
	}
</script>

<Section title="폰" lead="폰 화면의 메신저 대화에 쓰는 설정">
	{#if s}
		<Group>
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
