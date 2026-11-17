<script lang="ts">
	import { onMount } from "svelte";
	import { RefreshCw } from "@lucide/svelte";
	import type { UpdateInfo } from "$ipc/bindings";
	import { Button, Select, Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { checkForUpdate, openRelease } from "$app/updateCheck";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => appSetting.load());
	const s = $derived(appSetting.value);

	// 새 버전 확인 (1-E-2): 누를 때만 확인하고, 내려받기는 릴리스 페이지에서 직접
	let update = $state<UpdateInfo | null>(null);
	let checking = $state(false);
	async function check() {
		checking = true;
		update = await checkForUpdate().finally(() => (checking = false));
	}
	const status = $derived.by(() => {
		if (!update) return null;
		if (update.newer) return `새 버전 ${update.latest} 있음 (지금 ${update.current})`;
		if (!update.latest) return `아직 공개된 릴리스 없음 (지금 ${update.current})`;
		return `최신 버전 사용 중 (${update.current})`;
	});
</script>

<Section title="기본 설정" lead="앱 전체에 걸리는 설정. 대화·폰 설정은 각 탭에 있음">
	{#if s}
		<Group>
			<Row label="언어" desc="다른 언어는 준비 중">
				<Select value="ko" disabled options={[{ value: "ko", label: "한국어" }]} aria-label="언어" />
			</Row>
		</Group>
		<Group title="업데이트">
			<Row label="새 버전 확인" desc={status ?? "GitHub 릴리스에서 새 버전이 있는지 봄. 내려받기는 릴리스 페이지에서 직접"}>
				{#if update?.newer}<Button size="sm" variant="primary" onclick={() => update && openRelease(update)}>릴리스 페이지 열기</Button>{/if}
				<Button size="sm" loading={checking} disabled={checking} onclick={check}>{#snippet icon()}<RefreshCw />{/snippet}업데이트 확인</Button>
			</Row>
			<Row label="앱을 켤 때 새 버전 확인" desc="켜면 시작할 때마다 GitHub에 한 번 물어보고, 새 버전이 있으면 알림. 자동으로 내려받지 않음" nowrap={true}>
				<Switch checked={s.checkUpdatesOnStart} label="앱을 켤 때 새 버전 확인" hideLabel onchange={(v) => appSetting.update({ checkUpdatesOnStart: v })} />
			</Row>
		</Group>
	{/if}
</Section>
