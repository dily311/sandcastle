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
	import TypingIndicator from "$features/character/TypingIndicator.svelte";
	import type { TypingIndicator_Serialize as TypingSetting, TypingStyle } from "$ipc/bindings";

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

	// 끝나면 알림 (5-E-17·18): 제목 글과 아이콘만 바꾼다
	type Notify = { enabled: boolean; title: string; icon: string | null };
	const notify = $derived<Notify>({ enabled: !!s?.doneNotify?.enabled, title: s?.doneNotify?.title ?? "", icon: s?.doneNotify?.icon ?? null });
	const setNotify = (patch: Partial<Notify>) => appSetting.update({ doneNotify: { ...notify, ...patch } });
	async function toggleNotify(on: boolean) {
		// 켤 때 권한을 묻는다 (Android). 거절하면 켜지 않는다
		if (on && !(await call(commands.requestNotifyPermission(), { source: "settings" }).catch(() => false))) return;
		await setNotify({ enabled: on });
	}
	async function pickIcon() {
		const [p] = await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "ico"] }], false);
		if (p) await setNotify({ icon: p });
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
			<Row label="메시지 시각 표시" desc="대화에서 메시지마다 보낸 시각 보이기" nowrap>
				<Switch checked={s.showTimestamps} label="메시지 시각 표시" hideLabel onchange={(v) => appSetting.update({ showTimestamps: v })} />
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
			<Row label="응답이 오면 알림" desc="앱이 가려져 있을 때만. 알림에는 대화 내용 없이 '누구의 응답이 도착함'만" nowrap>
				<Switch checked={notify.enabled} label="응답이 오면 알림" hideLabel onchange={(v) => void toggleNotify(v)} />
			</Row>
			{#if notify.enabled}
				<Row label="알림 제목">
					<Input value={notify.title} placeholder="샌드캐슬 · {"{{char}}"} = 캐릭터 이름" maxlength={80} aria-label="알림 제목" onchange={(e) => setNotify({ title: e.currentTarget.value })} />
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
</style>
