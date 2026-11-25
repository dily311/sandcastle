<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { RefreshCw } from "@lucide/svelte";
	import type { Voice } from "$ipc/bindings";
	import { Button, Input, Select, Slider, Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { library } from "$features/character/library.svelte";
	import { speak } from "$shared/stores/aiTools.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const profileId = $derived(cur?.tts.apiProfileId ?? "");
	const profile = $derived(library.apis.find((a) => a.id === profileId));
	const eleven = $derived(profile?.formatType === "elevenlabs");

	// 목소리 목록: ElevenLabs는 계정의 목소리를 불러옴
	let voices = $state<Voice[]>([]);
	let loadingVoices = $state(false);
	async function loadVoices() {
		if (!profileId) return void (voices = []);
		loadingVoices = true;
		voices = await call(commands.listTtsVoices(profileId), { source: "tts" }).catch(() => []);
		loadingVoices = false;
	}
	$effect(() => {
		void profileId;
		void loadVoices();
	});
	const voiceOptions = $derived([
		{ value: "", label: eleven ? $_("settings.ttsSec.voiceSelect") : $_("common.default") },
		...voices.map((v) => ({ value: v.id, label: v.description ? `${v.name} · ${v.description}` : v.name })),
	]);
	const MODELS: Record<string, string[]> = {
		elevenlabs: ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_v3"],
		openai: ["tts-1", "tts-1-hd", "gpt-4o-mini-tts"],
	};
	const modelOptions = $derived((MODELS[eleven ? "elevenlabs" : "openai"] ?? []).map((m) => ({ value: m, label: m })));
</script>

<Section title={$_("settings.ttsSec.title")} lead={$_("settings.ttsSec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.ttsSec.api")} desc={$_("settings.ttsSec.apiDesc")}>
				<Select value={profileId} options={apiOptions($_("settings.chatSec.notSelected"), ["elevenlabs", "openai"])} onchange={(e) => save("tts", { apiProfileId: orNull(e.currentTarget.value), voice: "" })} />
			</Row>
			<Row label={$_("settings.ttsSec.model")}>
				{#if modelOptions.some((m) => m.value === cur.tts.model)}
					<Select value={cur.tts.model} options={modelOptions} onchange={(e) => save("tts", { model: e.currentTarget.value })} />
				{:else}
					<Input value={cur.tts.model} onchange={(e) => save("tts", { model: e.currentTarget.value.trim() })} />
				{/if}
			</Row>
			<Row label={$_("settings.ttsSec.voice")}>
				<div class="voice">
					<Select value={cur.tts.voice} options={voiceOptions} disabled={!profileId} onchange={(e) => save("tts", { voice: e.currentTarget.value })} />
					<Button size="sm" variant="ghost" loading={loadingVoices} disabled={!profileId} onclick={loadVoices}>{#snippet icon()}<RefreshCw />{/snippet}</Button>
				</div>
			</Row>
			<div class="slider">
				<Slider
					label={$_("settings.ttsSec.speed")}
					hint={eleven ? "ElevenLabs: 0.7–1.2x" : undefined}
					bind:value={() => String(cur.tts.speed), (v) => save("tts", { speed: Number(v) || 1 })}
					min={eleven ? 0.7 : 0.25}
					max={eleven ? 1.2 : 4}
					step={0.05}
					fallback={1}
					format={(n) => `${n.toFixed(2)}x`}
				/>
			</div>
			<Row label={$_("settings.ttsSec.dialogueOnly")} desc={$_("settings.ttsSec.dialogueOnlyDesc")}>
				<Switch checked={cur.tts.dialogueOnly} label={$_("settings.ttsSec.dialogueOnly")} hideLabel onchange={(v) => save("tts", { dialogueOnly: v })} />
			</Row>
			<Row label={$_("settings.ttsSec.autoRead")}>
				<Switch checked={cur.tts.autoRead} label={$_("settings.ttsSec.autoRead")} hideLabel onchange={(v) => save("tts", { autoRead: v })} />
			</Row>
			<Row label={$_("settings.ttsSec.sample")}>
				<Button size="sm" disabled={!profileId} onclick={() => speak({ id: "tts-test", content: "Hello. Testing the selected voice." })}>{$_("settings.ttsSec.preview")}</Button>
			</Row>
		</Group>
	{/if}
</Section>

<style>
	.voice {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.slider {
		padding: var(--space-2) var(--space-3);
	}
</style>
