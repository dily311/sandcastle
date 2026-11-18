<script lang="ts">
	import { onMount } from "svelte";
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
		{ value: "", label: eleven ? "목소리 선택" : "기본" },
		...voices.map((v) => ({ value: v.id, label: v.description ? `${v.name} · ${v.description}` : v.name })),
	]);
	const MODELS: Record<string, string[]> = {
		elevenlabs: ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_v3"],
		openai: ["tts-1", "tts-1-hd", "gpt-4o-mini-tts"],
	};
	const modelOptions = $derived((MODELS[eleven ? "elevenlabs" : "openai"] ?? []).map((m) => ({ value: m, label: m })));
</script>

<Section title="음성 읽기" lead="AI 답변을 소리 내어 읽음. ElevenLabs 또는 OpenAI 호환 음성 API">
	{#if cur}
		<Group>
			<Row label="음성 API" desc="API 키 화면에서 'ElevenLabs (음성)' 연결을 만들어 고름">
				<Select value={profileId} options={apiOptions("미선택", ["elevenlabs", "openai"])} onchange={(e) => save("tts", { apiProfileId: orNull(e.currentTarget.value), voice: "" })} />
			</Row>
			<Row label="모델">
				{#if modelOptions.some((m) => m.value === cur.tts.model)}
					<Select value={cur.tts.model} options={modelOptions} onchange={(e) => save("tts", { model: e.currentTarget.value })} />
				{:else}
					<Input value={cur.tts.model} onchange={(e) => save("tts", { model: e.currentTarget.value.trim() })} />
				{/if}
			</Row>
			<Row label="목소리" desc={eleven ? "계정에 있는 목소리 (Voice Library에서 추가한 것 포함)" : undefined}>
				<div class="voice">
					<Select value={cur.tts.voice} options={voiceOptions} disabled={!profileId} onchange={(e) => save("tts", { voice: e.currentTarget.value })} />
					<Button size="sm" variant="ghost" loading={loadingVoices} disabled={!profileId} onclick={loadVoices}>{#snippet icon()}<RefreshCw />{/snippet}</Button>
				</div>
			</Row>
			<div class="slider">
				<Slider
					label="속도"
					hint={eleven ? "ElevenLabs는 0.7~1.2배" : undefined}
					bind:value={() => String(cur.tts.speed), (v) => save("tts", { speed: Number(v) || 1 })}
					min={eleven ? 0.7 : 0.25}
					max={eleven ? 1.2 : 4}
					step={0.05}
					fallback={1}
					format={(n) => `${n.toFixed(2)}배`}
				/>
			</div>
			<Row label="대사만 읽기" desc="*지문*은 건너뜀">
				<Switch checked={cur.tts.dialogueOnly} label="대사만 읽기" hideLabel onchange={(v) => save("tts", { dialogueOnly: v })} />
			</Row>
			<Row label="새 답변 자동 읽기">
				<Switch checked={cur.tts.autoRead} label="자동 읽기" hideLabel onchange={(v) => save("tts", { autoRead: v })} />
			</Row>
			<Row label="들어 보기">
				<Button size="sm" disabled={!profileId} onclick={() => speak({ id: "tts-test", content: "안녕하세요. 지금 고른 목소리로 읽고 있어요." })}>미리 듣기</Button>
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
