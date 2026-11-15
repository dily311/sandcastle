<script lang="ts">
	// 코어 플러그인: 부가 기능을 껐다 켬 (Obsidian의 코어 플러그인처럼). 목록과 기본값은 백엔드 sys/plugins.rs
	import { onMount } from "svelte";
	import { Badge, Input, Select, Slider, Switch, Textarea } from "$shared/ui";
	import type { PluginSetting } from "$ipc/bindings";
	import { library } from "$features/character/library.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";
	import { layout } from "$app/layout.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => {
		void plugins.load();
		void pluginSetting.load();
		if (!library.loaded) void library.load();
	});

	// ---- AI 도구 세부 설정 (켜진 것만 보임) ----
	const apiOptions = $derived([{ value: "", label: "앱에서 고른 API" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const VOICES = ["alloy", "ash", "ballad", "coral", "echo", "fable", "nova", "onyx", "sage", "shimmer"].map((v) => ({ value: v, label: v }));
	const SIZES = ["1024x1024", "1024x1536", "1536x1024", "512x512", "auto"].map((v) => ({ value: v, label: v }));
	const cur = $derived(pluginSetting.value as Required<PluginSetting> | null);
	type Section = "translate" | "tts" | "imageGen";
	function set<K extends Section>(section: K, patch: Partial<Required<PluginSetting>[K]>) {
		if (!cur) return;
		void pluginSetting.save({ ...cur, [section]: { ...cur[section], ...patch } } as PluginSetting);
	}
	const profile = (v: string) => (v ? v : null);

	const GROUPS: { title: string; ids: PluginId[] }[] = [
		{ title: "화면", ids: ["phone", "game", "widget"] },
		{ title: "대화 표시", ids: ["chat.images", "html", "regex"] },
		{ title: "AI 도구", ids: ["translate", "tts", "imagegen"] },
	];
	const byId = (id: string) => plugins.list.find((p) => p.id === id);
</script>

<Section title="코어 플러그인" lead="필요 없는 부가 기능은 꺼 두면 메뉴와 설정에서 사라지고 AI에게도 쓰이지 않음. 끈 기능의 데이터는 지우지 않음">
	{#each GROUPS as g (g.title)}
		<Group title={g.title}>
			{#each g.ids as id (id)}
				{@const p = byId(id)}
				{#if p}
					<Row label={p.name} desc={p.description}>
						{#if p.desktopOnly && layout.mobile}<Badge>데스크톱 전용</Badge>{/if}
						<Switch checked={p.enabled} label="{p.name} 사용" hideLabel disabled={p.desktopOnly && layout.mobile} onchange={(v) => plugins.set(id, v)} />
					</Row>
				{/if}
			{/each}
		</Group>
	{/each}

	{#if cur}
		{#if plugins.on("translate")}
			<Group title="번역 설정">
				<Row label="번역에 쓸 API" desc="채팅 모델로 번역함. 싸고 빠른 모델이면 충분">
					<Select value={cur.translate.apiProfileId ?? ""} options={apiOptions} onchange={(e) => set("translate", { apiProfileId: profile(e.currentTarget.value) })} />
				</Row>
				<Row label="AI 답변을 이 언어로">
					<Input value={cur.translate.targetLang} onchange={(e) => set("translate", { targetLang: e.currentTarget.value.trim() || "한국어" })} />
				</Row>
				<Row label="내 입력을 이 언어로" desc="입력창 + 메뉴의 '입력을 번역해서 바꾸기'">
					<Input value={cur.translate.inputLang} onchange={(e) => set("translate", { inputLang: e.currentTarget.value.trim() || "English" })} />
				</Row>
				<Row label="새 답변 자동 번역" desc="답변이 오면 바로 번역해서 보여줌 (원문 보기로 되돌림)">
					<Switch checked={cur.translate.autoTranslate} label="자동 번역" hideLabel onchange={(v) => set("translate", { autoTranslate: v })} />
				</Row>
			</Group>
		{/if}
		{#if plugins.on("tts")}
			<Group title="음성 읽기 설정">
				<Row label="음성 API" desc="OpenAI 호환 /audio/speech를 쓰는 연결 (주소와 키)">
					<Select value={cur.tts.apiProfileId ?? ""} options={apiOptions} onchange={(e) => set("tts", { apiProfileId: profile(e.currentTarget.value) })} />
				</Row>
				<Row label="모델"><Input value={cur.tts.model} onchange={(e) => set("tts", { model: e.currentTarget.value.trim() || "tts-1" })} /></Row>
				<Row label="목소리">
					<Select value={cur.tts.voice} options={VOICES} onchange={(e) => set("tts", { voice: e.currentTarget.value })} />
				</Row>
				<div class="slider">
					<Slider label="속도" bind:value={() => String(cur.tts.speed), (v) => set("tts", { speed: Number(v) || 1 })} min={0.25} max={4} step={0.05} fallback={1} format={(n) => `${n.toFixed(2)}배`} />
				</div>
				<Row label="대사만 읽기" desc="*지문*은 건너뜀">
					<Switch checked={cur.tts.dialogueOnly} label="대사만 읽기" hideLabel onchange={(v) => set("tts", { dialogueOnly: v })} />
				</Row>
				<Row label="새 답변 자동 읽기">
					<Switch checked={cur.tts.autoRead} label="자동 읽기" hideLabel onchange={(v) => set("tts", { autoRead: v })} />
				</Row>
			</Group>
		{/if}
		{#if plugins.on("imagegen")}
			<Group title="이미지 생성 설정">
				<Row label="이미지 API" desc="OpenAI 호환 /images/generations를 쓰는 연결 (주소와 키)">
					<Select value={cur.imageGen.apiProfileId ?? ""} options={apiOptions} onchange={(e) => set("imageGen", { apiProfileId: profile(e.currentTarget.value) })} />
				</Row>
				<Row label="모델"><Input value={cur.imageGen.model} onchange={(e) => set("imageGen", { model: e.currentTarget.value.trim() || "gpt-image-1" })} /></Row>
				<Row label="크기">
					<Select value={cur.imageGen.size} options={SIZES} onchange={(e) => set("imageGen", { size: e.currentTarget.value })} />
				</Row>
				<Row label="AI가 그림 프롬프트 쓰기" desc="메시지를 그대로 보내지 않고, 채팅 AI가 장면을 영어 프롬프트로 정리함">
					<Switch checked={cur.imageGen.aiPrompt} label="AI 프롬프트" hideLabel onchange={(v) => set("imageGen", { aiPrompt: v })} />
				</Row>
				<Row label="화풍" desc="모든 그림 프롬프트 끝에 붙음" stacked>
					<Textarea value={cur.imageGen.stylePrompt} rows={2} placeholder="예: anime style, soft lighting, detailed background" onchange={(e) => set("imageGen", { stylePrompt: e.currentTarget.value })} />
				</Row>
			</Group>
		{/if}
	{/if}
</Section>

<style>
	.slider {
		padding: var(--space-2) var(--space-3);
	}
</style>
