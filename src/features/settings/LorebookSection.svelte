<script lang="ts">
	// 로어북(월드인포) 설정: 스캔 깊이 · 재귀 최대 깊이
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Input } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => void appSetting.load());

	const scanDepth = $derived(appSetting.value?.worldInfo?.scanDepth ?? 2);
	const maxRecursionDepth = $derived(appSetting.value?.worldInfo?.maxRecursionDepth ?? 0);

	async function saveScanDepth(raw: string) {
		const n = Math.min(200, Math.max(1, Math.round(Number(raw) || 2)));
		await appSetting.update({
			worldInfo: { scanDepth: n, maxRecursionDepth },
		});
		toasts.success(`월드인포 스캔 깊이를 ${n}개로 저장함`);
	}

	async function saveMaxDepth(raw: string) {
		const n = Math.min(10, Math.max(0, Math.round(Number(raw) || 0)));
		await appSetting.update({
			worldInfo: { scanDepth, maxRecursionDepth: n },
		});
		toasts.success(n === 0 ? "월드인포 재귀 스캔을 끔 (최대 깊이 0)" : `월드인포 최대 재귀 깊이를 ${n}로 저장함`);
	}
</script>

<Section title={$_("settings.lorebook.title")} lead={$_("settings.lorebook.lead")}>
	<Group title={$_("settings.lorebook.scanGroup")}>
		<Row label={$_("settings.lorebook.scanDepth")} desc={$_("settings.lorebook.scanDepthDesc")}>
			<div class="num">
				<Input
					type="number"
					min="1"
					max="200"
					value={String(scanDepth)}
					aria-label={$_("settings.lorebook.scanDepth")}
					onchange={(e) => saveScanDepth(e.currentTarget.value)}
				/>
			</div>
		</Row>
		<Row label={$_("settings.lorebook.maxDepth")} desc={$_("settings.lorebook.maxDepthDesc")}>
			<div class="num">
				<Input
					type="number"
					min="0"
					max="10"
					value={String(maxRecursionDepth)}
					aria-label={$_("settings.lorebook.maxDepth")}
					onchange={(e) => saveMaxDepth(e.currentTarget.value)}
				/>
			</div>
		</Row>
	</Group>
</Section>

<style>
	.num {
		width: 110px;
	}
</style>
