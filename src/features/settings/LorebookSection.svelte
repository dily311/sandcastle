<script lang="ts">
	// 로어북(월드인포) 설정: 스캔 깊이 · 재귀 최대 깊이
	import { onMount } from "svelte";
	import { Input } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => void appSetting.load());

	const scanDepth = $derived(appSetting.value?.worldInfo?.scanDepth ?? 10);
	const maxRecursionDepth = $derived(appSetting.value?.worldInfo?.maxRecursionDepth ?? 0);

	async function saveScanDepth(raw: string) {
		const n = Math.min(200, Math.max(1, Math.round(Number(raw) || 10)));
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

<Section title="로어북" lead="월드인포 키워드 탐색 범위(스캔 깊이)와 연쇄 발동(최대 재귀 깊이)을 설정함">
	<Group title="스캔 설정">
		<Row
			label="스캔 깊이 (Scan Depth)"
			desc="월드인포 키워드를 찾을 최근 대화 메시지 수 (1~200, 기본 10). 이번 사용자 입력은 항상 포함됨"
		>
			<div class="num">
				<Input
					type="number"
					min="1"
					max="200"
					value={String(scanDepth)}
					aria-label="월드인포 스캔 깊이"
					onchange={(e) => saveScanDepth(e.currentTarget.value)}
				/>
			</div>
		</Row>
		<Row
			label="최대 재귀 깊이 (Max Depth)"
			desc="발동된 월드인포 항목의 내용에서 다른 월드인포 키워드를 연쇄 탐색할 최대 깊이 (0~10, 0이면 재귀 안 함)"
		>
			<div class="num">
				<Input
					type="number"
					min="0"
					max="10"
					value={String(maxRecursionDepth)}
					aria-label="월드인포 최대 재귀 깊이"
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
