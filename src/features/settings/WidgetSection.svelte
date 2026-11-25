<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Segmented, Switch } from "$shared/ui";
	import { pickFolder } from "$shared/ipc/dialog";
	import type { WidgetMusicSetting_Serialize } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { WIDGET_FEATURES } from "$lib/features/setFeautres";
	import { widgetFeatureConfig } from "$lib/features/widgetFeatures.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import WidgetMascotSettings from "./WidgetMascotSettings.svelte";

	const src = { source: "widget" };
	let tab = $state<"general" | "shimeji">("general");
	let features = $state<Record<string, boolean>>({});
	let autostart = $state(false);
	let ready = $state(false);
	// 음악 플레이어 설정 (5-E-21): 위젯 창과 같은 저장소 (바꾸면 위젯 창이 이벤트로 따라감)
	let music = $state<WidgetMusicSetting_Serialize | null>(null);

	async function setMusic(patch: Partial<WidgetMusicSetting_Serialize>) {
		if (!music) return;
		music = { ...music, ...patch };
		await call(commands.saveWidgetMusicSettings(patch as never), src);
	}
	async function chooseMusicFolder() {
		const dir = await pickFolder();
		if (dir) await setMusic({ selectedFolder: dir });
	}

	onMount(async () => {
		const [f, a] = await Promise.all([
			call(commands.getFeatureSetting(), { ...src, silent: true }).catch(() => ({ feature: {} })),
			call(commands.isAutostartEnabled(), { ...src, silent: true }).catch(() => false),
		]);
		features = { ...f.feature };
		music = await call(commands.getWidgetMusicSettings(), { ...src, silent: true }).catch(() => null);
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
		<div class="tabs">
			<Segmented
				label="위젯 설정 탭"
				bind:value={tab}
				options={[
					{ value: "general", label: "일반" },
					{ value: "shimeji", label: "시메지" },
				]}
			/>
		</div>

		{#if tab === "general"}
			<Group>
				<Row label="컴퓨터 켤 때 위젯 실행" desc="로그인하면 위젯만 먼저 띄움">
					<Switch checked={autostart} label="컴퓨터 켤 때 위젯 실행" hideLabel onchange={setAutostart} />
				</Row>
			</Group>
			<Group title="위젯 기능">
				{#each WIDGET_FEATURES as f (f.id)}
					<Row label={f.name} desc={f.description.replace(/합니다\.$/, "함")}>
						<Switch
							checked={features[f.id] ?? f.defaultEnabled}
							label={f.name}
							hideLabel
							disabled={!features["widget"]}
							onchange={(v) => toggle(f.id, v)}
						/>
					</Row>
				{/each}
			</Group>
			{#if music && features["widget"] && features["widget.music"]}
				<Group title="음악 플레이어">
					<Row label="작게 보기">
						<Switch checked={music.isMiniPlayer} label="작게 보기" hideLabel onchange={(v) => void setMusic({ isMiniPlayer: v })} />
					</Row>
					<Row label="음악 폴더" desc={music.selectedFolder ?? "고르지 않음"}>
						<Button size="sm" onclick={chooseMusicFolder}>바꾸기</Button>
					</Row>
					<Row label="위치">
						<Button size="sm" variant="ghost" disabled={!music.pos} onclick={() => void setMusic({ pos: null })}>처음 자리로</Button>
					</Row>
				</Group>
			{/if}
		{:else}
			<WidgetMascotSettings />
		{/if}
	{/if}
</Section>

<style>
	.tabs {
		margin-bottom: var(--space-3);
	}
</style>
