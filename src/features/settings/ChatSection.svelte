<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Input, Select, Switch } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { pickFiles } from "$shared/ipc/dialog";
	import { layout } from "$app/layout.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import ModelPicker from "./ModelPicker.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import TypingIndicator from "$features/character/TypingIndicator.svelte";
	import type { DoneNotify_Serialize as NotifySetting, NotifySoundMode, TypingIndicator_Serialize as TypingSetting, TypingStyle } from "$ipc/bindings";
	import ProfilePicker, { type ProfileOption } from "$/shared/ui/ProfilePicker.svelte";
	import { playNotifySound } from "$app/notifySound";

	let defaultPersona = $state<string>("");

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? null : library.load()]);
		const conv = await call(commands.getConvSettings(), { source: "settings", silent: true }).catch(() => null);
		defaultPersona = conv?.defaultPersonaId ?? "default";
	});

	const s = $derived(appSetting.value);
	const apiOptions = $derived([{ value: "", label: "고르지 않음" }, ...library.apis.map((a) => ({ value: a.id, label: a.model ? `${a.name} · ${a.model}` : `${a.name} (${a.provider})` }))]);
	const selectedApi = $derived(library.apis.find((a) => a.id === (s?.selectedApiProfile ?? "")));
	const presetOptions = $derived(library.presets.map((p) => ({ value: p.id, label: p.name })));
	const personaOptions = $derived<ProfileOption[]>(library.personas.map((p) => ({ id: p.id, name: p.name, avatar: p.avatar, avatarFit: p.avatarFit, color: p.color, description: p.description })));

	// 끝나면 알림 (5-E-17·18): 제목 글, 아이콘, 소리
	const notify = $derived<NotifySetting>({
		enabled: !!s?.doneNotify?.enabled,
		title: s?.doneNotify?.title ?? "",
		icon: s?.doneNotify?.icon ?? null,
		soundMode: s?.doneNotify?.soundMode ?? "default",
		soundPath: s?.doneNotify?.soundPath ?? null,
		soundVolume: s?.doneNotify?.soundVolume ?? 0.8,
	});
	const notifySoundModes: { value: NotifySoundMode; label: string }[] = [
		{ value: "default", label: "기본 알림음" },
		{ value: "mute", label: "무음" },
		{ value: "custom", label: "직접 고른 소리" },
	];
	const soundFileName = $derived(notify.soundPath ? (notify.soundPath.split(/[\\/]/).pop() ?? notify.soundPath) : null);
	const setNotify = (patch: Partial<NotifySetting>) => appSetting.update({ doneNotify: { ...notify, ...patch } });
	async function toggleNotify(on: boolean) {
		// 켤 때 권한을 묻는다 (Android). 거절하면 켜지 않는다
		if (on && !(await call(commands.requestNotifyPermission(), { source: "settings" }).catch(() => false))) return;
		await setNotify({ enabled: on });
	}
	async function pickIcon() {
		const [p] = await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "ico"] }], false);
		if (p) await setNotify({ icon: p });
	}
	async function pickNotifySound() {
		const [p] = await pickFiles([{ name: "오디오", extensions: ["mp3", "wav", "ogg", "m4a", "flac", "webm", "opus"] }], false);
		if (p) appSetting.value = await call(commands.setNotifySound(p), { source: "settings" });
	}
	async function clearNotifySound() {
		appSetting.value = await call(commands.setNotifySound(null), { source: "settings" });
	}
	async function sendTestNotify() {
		await call(commands.testNotify(), { source: "settings" });
	}

	// 타이핑 표시 (5-E-17-1)
	const typing = $derived<TypingSetting>({ style: s?.typing?.style ?? "blink", text: s?.typing?.text ?? "", image: s?.typing?.image ?? null });
	const typingStyles: { value: TypingStyle; label: string }[] = [
		{ value: "blink", label: "깜빡이는 점" },
		{ value: "bounce", label: "튀는 점" },
		{ value: "none", label: "점 없음" },
		{ value: "image", label: "그림·GIF" },
	];
	async function pickTypingImage() {
		const [p] = await pickFiles([{ name: "그림", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], false);
		if (p) appSetting.value = await call(commands.setTypingImage(p), { source: "settings" });
	}
	async function clearTypingImage() {
		appSetting.value = await call(commands.setTypingImage(null), { source: "settings" });
	}
	const setTyping = (patch: Partial<TypingSetting>) => appSetting.update({ typing: { ...typing, ...patch } });

	async function setPersona(id: string) {
		defaultPersona = id;
		await call(commands.setSystemDefaultPersona(id), { source: "settings" });
	}
</script>

<Section title="대화" lead="대화 화면과 새 대화에 쓰는 설정">
	{#if s}
		<Group>
			<Row label="Enter로 보내기" desc="끄면 Ctrl+Enter로 보냄. Shift+Enter는 항상 줄바꿈" nowrap>
				<Switch checked={s.sendOnEnter} label="Enter로 보내기" hideLabel onchange={(v) => appSetting.update({ sendOnEnter: v })} />
			</Row>
		</Group>

		<Group title="타이핑 표시">
			<Row label="모양">
				<div class="typing-row">
					<span class="typing-preview"><TypingIndicator style={typing.style} text={typing.text} image={typing.image} name="엘리안" /></span>
					<Select value={typing.style} options={typingStyles} aria-label="타이핑 표시 모양" onchange={(e) => setTyping({ style: e.currentTarget.value as TypingStyle })} />
				</div>
			</Row>
			{#if typing.style === "image"}
				<Row label="그림">
					<Button size="sm" variant="secondary" onclick={pickTypingImage}>{typing.image ? "바꾸기" : "고르기"}</Button>
					{#if typing.image}<Button size="sm" variant="ghost" onclick={clearTypingImage}>빼기</Button>{/if}
				</Row>
			{/if}
			<Row label="글">
				<Input value={typing.text} placeholder={"{{char}} 입력 중…"} maxlength={60} aria-label="타이핑 표시 글" onchange={(e) => setTyping({ text: e.currentTarget.value })} />
			</Row>
		</Group>

		<Group title="알림">
			<Row label="작업이 끝나면 알림" desc="앱이 가려져 있을 때만. 알림에는 대화 내용 없이 완료 안내만" nowrap>
				<Switch checked={notify.enabled} label="작업이 끝나면 알림" hideLabel onchange={(v) => void toggleNotify(v)} />
			</Row>
			{#if notify.enabled}
				<Row label="알림 제목">
					<Input value={notify.title} placeholder="샌드캐슬 · {"{{char}}"} = 캐릭터·작업 이름" maxlength={80} aria-label="알림 제목" onchange={(e) => setNotify({ title: e.currentTarget.value })} />
				</Row>
				{#if !layout.mobile}
					<Row label="알림 아이콘">
						<div class="icon-row">
							{#if notify.icon}<img src={fileSrc(notify.icon)} alt="알림 아이콘" />{/if}
							<Button size="sm" onclick={pickIcon}>그림 고르기</Button>
							{#if notify.icon}<Button size="sm" variant="ghost" onclick={() => setNotify({ icon: null })}>기본으로</Button>{/if}
						</div>
					</Row>
				{/if}
				<Row label="알림 소리">
					<Select
						value={notify.soundMode}
						options={notifySoundModes}
						aria-label="알림 소리 방식"
						onchange={(e) => setNotify({ soundMode: e.currentTarget.value as NotifySoundMode })}
					/>
				</Row>
				{#if notify.soundMode === "custom"}
					<Row label="소리 파일">
						<div class="sound-row">
							{#if soundFileName}<span class="sound-name" title={soundFileName}>{soundFileName}</span>{/if}
							<Button size="sm" variant="secondary" onclick={pickNotifySound}>{notify.soundPath ? "바꾸기" : "고르기"}</Button>
							{#if notify.soundPath}
								<Button size="sm" variant="ghost" onclick={() => notify.soundPath && playNotifySound(notify.soundPath, notify.soundVolume)}>미리듣기</Button>
								<Button size="sm" variant="ghost" onclick={clearNotifySound}>빼기</Button>
							{/if}
						</div>
					</Row>
					<Row label="소리 크기">
						<div class="volume-row">
							<input
								type="range"
								min="0"
								max="1"
								step="0.05"
								value={notify.soundVolume ?? 0.8}
								aria-label="알림 소리 크기"
								onchange={(e) => setNotify({ soundVolume: Number(e.currentTarget.value) })}
							/>
							<span class="volume-val">{Math.round((notify.soundVolume ?? 0.8) * 100)}%</span>
						</div>
					</Row>
				{/if}
				<Row label="테스트">
					<Button size="sm" variant="secondary" onclick={sendTestNotify}>알림 테스트</Button>
				</Row>
			{/if}
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
			{#if selectedApi}
				<Row label="모델" desc="대화 설정에서 따로 고르지 않으면 이걸 씀">
					<ModelPicker
						profile={selectedApi}
						value={s.selectedModel ?? null}
						onchange={(m) => appSetting.update({ selectedModel: m })}
						effort={s.reasoningEffort ?? "auto"}
						oneffort={(e) => appSetting.update({ reasoningEffort: e === "auto" ? null : e })}
						label="기본 모델"
					/>
				</Row>
			{/if}
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
				<ProfilePicker label="페르소나 변경" options={personaOptions} value={defaultPersona} onchange={(id) => void setPersona(id)} />
			</Row>
		</Group>

		{#if plugins.on("status")}
			<Group title="AI 변수 프롬프트">
				<PromptSlotEditor key="vars" desc="'AI가 변수 읽고 바꾸기'를 켰을 때 붙음" vars={"{{var_list}} 변수 목록·지금 값 · 프리셋 블록에 {{vars}}를 넣은 자리에 들어감"} />
			</Group>
		{/if}
	{/if}
</Section>

<style>
	.typing-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.typing-preview {
		flex-shrink: 0;
		white-space: nowrap;
	}
	.icon-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.icon-row img {
		width: 28px;
		height: 28px;
		border-radius: var(--radius-sm);
		object-fit: cover;
	}
	.sound-row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.sound-name {
		max-width: 180px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.volume-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.volume-row input[type="range"] {
		width: 140px;
		accent-color: var(--accent);
	}
	.volume-val {
		min-width: 36px;
		font-size: var(--text-xs);
		color: var(--text-muted);
		text-align: right;
	}
</style>
