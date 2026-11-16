<script lang="ts">
	import { onMount } from "svelte";
	import { Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { WIDGET_FEATURES } from "$lib/features/setFeautres";
	import { widgetFeatureConfig } from "$lib/features/widgetFeatures.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "widget" };
	let features = $state<Record<string, boolean>>({});
	let autostart = $state(false);
	let ready = $state(false);

	onMount(async () => {
		const [f, a] = await Promise.all([
			call(commands.getFeatureSetting(), { ...src, silent: true }).catch(() => ({ feature: {} })),
			call(commands.isAutostartEnabled(), { ...src, silent: true }).catch(() => false),
		]);
		features = { ...f.feature };
		autostart = a;
		ready = true;
	});

	async function toggle(id: string, on: boolean) {
		features[id] = on;
		// 위젯 창 자체는 백엔드가 열고 닫음 (update_feature_setting). 위젯 하위 기능은 위젯 창이 이벤트로 반영
		await call(commands.updateFeatureSetting(id, on), src);
		widgetFeatureConfig.value.set(id, on);
	}
	async function setAutostart(on: boolean) {
		autostart = on;
		await call(commands.setAutostart(on), src).catch(() => (autostart = !on));
	}
</script>

<Section title="위젯" lead="바탕화면에 떠 있는 작은 창">
	{#if ready}
		<Group>
			<Row label="컴퓨터 켤 때 위젯 실행" desc="로그인하면 위젯만 먼저 띄움">
				<Switch checked={autostart} label="컴퓨터 켤 때 위젯 실행" hideLabel onchange={setAutostart} />
			</Row>
		</Group>
		<Group title="위젯 기능">
			{#each WIDGET_FEATURES as f (f.id)}
				<Row label={f.name} desc={f.description.replace(/합니다\.$/, "함")}>
					<Switch checked={features[f.id] ?? f.defaultEnabled} label={f.name} hideLabel disabled={!features["widget"]} onchange={(v) => toggle(f.id, v)} />
				</Row>
			{/each}
		</Group>
	{/if}
</Section>
