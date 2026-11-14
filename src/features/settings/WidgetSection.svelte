<script lang="ts">
	import { onMount } from "svelte";
	import { Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { WIDGET_FEATURES } from "$lib/features/setFeautres";
	import { featureConfig, featureManager } from "$lib/features/feautreManager.svelte";
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
		// 위젯 창 자체는 기능 관리자가 열고 닫음. 나머지는 설정만 바꾸면 위젯 창이 알아서 반영
		if (id === "widget" && featureConfig.value.has(id)) await featureManager.toggleFeature(id, on);
		else {
			await call(commands.updateFeatureSetting(id, on), src);
			featureConfig.value.set(id, on);
		}
	}
	async function setAutostart(on: boolean) {
		autostart = on;
		await call(commands.setAutostart(on), src).catch(() => (autostart = !on));
	}
</script>

<Section title="위젯" lead="바탕화면에 떠 있는 작은 창">
	{#if ready}
		<Group>
			<Row label="위젯 켜기" desc="켜면 위젯 창이 열림">
				<Switch checked={features["widget"] ?? false} label="위젯 켜기" hideLabel onchange={(v) => toggle("widget", v)} />
			</Row>
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
