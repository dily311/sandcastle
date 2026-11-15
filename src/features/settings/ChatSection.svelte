<script lang="ts">
	import { onMount } from "svelte";
	import { Select, Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	let defaultPersona = $state<string>("");

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? null : library.load()]);
		const conv = await call(commands.getConvSettings(), { source: "settings", silent: true }).catch(() => null);
		defaultPersona = conv?.defaultPersonaId ?? "default";
	});

	const s = $derived(appSetting.value);
	const apiOptions = $derived([{ value: "", label: "고르지 않음" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const presetOptions = $derived(library.presets.map((p) => ({ value: p.id, label: p.name })));
	const personaOptions = $derived(library.personas.map((p) => ({ value: p.id, label: p.name })));

	async function setPersona(id: string) {
		defaultPersona = id;
		await call(commands.setSystemDefaultPersona(id), { source: "settings" });
	}
</script>

<Section title="대화" lead="대화 화면과 새 대화에 쓰는 설정">
	{#if s}
		<Group>
			<Row label="Enter로 보내기" desc="끄면 Ctrl+Enter로 보냄. Shift+Enter는 항상 줄바꿈">
				<Switch checked={s.sendOnEnter} label="Enter로 보내기" hideLabel onchange={(v) => appSetting.update({ sendOnEnter: v })} />
			</Row>
			<Row label="메시지 시각 표시" desc="대화에서 메시지마다 보낸 시각 보이기">
				<Switch checked={s.showTimestamps} label="메시지 시각 표시" hideLabel onchange={(v) => appSetting.update({ showTimestamps: v })} />
			</Row>
		</Group>

		<Group title="새 대화 기본값">
			<Row label="API" desc="대화 설정에서 따로 고르지 않으면 이걸 씀">
				<Select
					value={s.selectedApiProfile ?? ""}
					options={apiOptions}
					aria-label="기본 API"
					onchange={(e) => appSetting.update({ selectedApiProfile: e.currentTarget.value || null })}
				/>
			</Row>
			<Row label="프리셋" desc="대화 설정에서 '앱 기본값'일 때 씀">
				<Select
					value={s.selectedPreset}
					options={presetOptions}
					placeholder="프리셋 없음"
					aria-label="기본 프리셋"
					onchange={(e) => appSetting.update({ selectedPreset: e.currentTarget.value })}
				/>
			</Row>
			<Row label="페르소나" desc="캐릭터에 기본 페르소나가 없을 때 씀">
				<Select value={defaultPersona} options={personaOptions} aria-label="기본 페르소나" onchange={(e) => setPersona(e.currentTarget.value)} />
			</Row>
		</Group>
	{/if}
</Section>
