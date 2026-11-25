<script lang="ts">
	// 설정 > 폰: 폰 화면 오른쪽 패널의 정보 탭과 같은 설정 (PhoneSettings)
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { phone } from "$features/phone/phone.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PhoneSettings from "$features/phone/PhoneSettings.svelte";

	// 폰 화면을 열지 않았어도 주인·연락처를 읽어 둔다 (주인 고르기에 필요)
	onMount(() => void phone.init());
</script>

<Section title={$_("settings.phoneSec.title")} lead={$_("settings.phoneSec.lead")}>
	<Group>
		<div class="pad"><PhoneSettings /></div>
	</Group>
</Section>

<style>
	.pad {
		padding: var(--space-4) 0;
	}
</style>
