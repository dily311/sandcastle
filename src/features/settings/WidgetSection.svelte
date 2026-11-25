<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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

<Section title={$_("settings.widgetSec.title")} lead={$_("settings.widgetSec.lead")}>
	{#if ready}
		<div class="tabs">
			<Segmented
				label={$_("settings.widgetSec.title")}
				bind:value={tab}
				options={[
					{ value: "general", label: $_("settings.widgetSec.tabGeneral") },
					{ value: "shimeji", label: $_("settings.widgetSec.tabShimeji") },
				]}
			/>
		</div>

		{#if tab === "general"}
			<Group>
				<Row label={$_("settings.widgetSec.autostart")} desc={$_("settings.widgetSec.autostartDesc")}>
					<Switch checked={autostart} label={$_("settings.widgetSec.autostart")} hideLabel onchange={setAutostart} />
				</Row>
			</Group>
			<Group title={$_("settings.widgetSec.featuresGroup")}>
				{#each WIDGET_FEATURES as f (f.id)}
					{@const fid = f.id.replaceAll(".", "_")}
					{@const fname = $_(`widgetExtra.features.${fid}.name`, { default: f.name })}
					{@const fdesc = $_(`widgetExtra.features.${fid}.desc`, { default: f.description.replace(/합니다\.$/, "함") })}
					<Row label={fname} desc={fdesc}>
						<Switch
							checked={features[f.id] ?? f.defaultEnabled}
							label={fname}
							hideLabel
							disabled={!features["widget"]}
							onchange={(v) => toggle(f.id, v)}
						/>
					</Row>
				{/each}
			</Group>
			{#if music && features["widget"] && features["widget.music"]}
				<Group title={$_("settings.widgetSec.musicGroup")}>
					<Row label={$_("settings.widgetSec.miniPlayer")}>
						<Switch checked={music.isMiniPlayer} label={$_("settings.widgetSec.miniPlayer")} hideLabel onchange={(v) => void setMusic({ isMiniPlayer: v })} />
					</Row>
					<Row label={$_("settings.widgetSec.musicFolder")} desc={music.selectedFolder ?? $_("settings.chatSec.notSelected")}>
						<Button size="sm" onclick={chooseMusicFolder}>{$_("settings.chatSec.change")}</Button>
					</Row>
					<Row label={$_("settings.widgetSec.position")}>
						<Button size="sm" variant="ghost" disabled={!music.pos} onclick={() => void setMusic({ pos: null })}>{$_("settings.widgetSec.resetPos")}</Button>
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
