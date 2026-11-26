<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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
	const apiOptions = $derived([{ value: "", label: $_("settings.chatSec.notSelected") }, ...library.apis.map((a) => ({ value: a.id, label: a.model ? `${a.name} · ${a.model}` : `${a.name} (${a.provider})` }))]);
	const selectedApi = $derived(library.apis.find((a) => a.id === (s?.selectedApiProfile ?? "")));
	const presetOptions = $derived(library.presets.map((p) => ({ value: p.id, label: p.name })));
	const personaOptions = $derived<ProfileOption[]>(library.personas.map((p) => ({ id: p.id, name: p.name, avatar: p.avatar, avatarFit: p.avatarFit, color: p.color, description: p.description })));

	// 끝나면 알림 (5-E-17·18, 1-J-5 기본 켜짐): 제목 글, 아이콘, 소리
	const notify = $derived<NotifySetting>({
		enabled: s?.doneNotify?.enabled ?? true,
		initializedOn: s?.doneNotify?.initializedOn ?? true,
		title: s?.doneNotify?.title ?? "",
		icon: s?.doneNotify?.icon ?? null,
		soundMode: s?.doneNotify?.soundMode ?? "default",
		soundPath: s?.doneNotify?.soundPath ?? null,
		soundVolume: s?.doneNotify?.soundVolume ?? 0.8,
	});
	const notifySoundModes = $derived<{ value: NotifySoundMode; label: string }[]>([
		{ value: "default", label: $_("settings.chatSec.soundDefault") },
		{ value: "mute", label: $_("settings.chatSec.soundMute") },
		{ value: "custom", label: $_("settings.chatSec.soundCustom") },
	]);
	const soundFileName = $derived(notify.soundPath ? (notify.soundPath.split(/[\\/]/).pop() ?? notify.soundPath) : null);
	const setNotify = (patch: Partial<NotifySetting>) => appSetting.update({ doneNotify: { ...notify, initializedOn: true, ...patch } });
	async function toggleNotify(on: boolean) {
		if (on) void call(commands.requestNotifyPermission(), { source: "settings", silent: true }).catch(() => false);
		await setNotify({ enabled: on, initializedOn: true });
	}
	async function pickIcon() {
		const [p] = await pickFiles([{ name: $_("settings.more.imgFilter"), extensions: ["png", "jpg", "jpeg", "ico"] }], false);
		if (p) await setNotify({ icon: p });
	}
	async function pickNotifySound() {
		const [p] = await pickFiles([{ name: $_("settings.more.audioFilter"), extensions: ["mp3", "wav", "ogg", "m4a", "flac", "webm", "opus"] }], false);
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
	const typingStyles = $derived<{ value: TypingStyle; label: string }[]>([
		{ value: "blink", label: $_("settings.chatSec.styleBlink") },
		{ value: "bounce", label: $_("settings.chatSec.styleBounce") },
		{ value: "none", label: $_("settings.chatSec.styleNone") },
		{ value: "image", label: $_("settings.chatSec.styleImage") },
	]);
	async function pickTypingImage() {
		const [p] = await pickFiles([{ name: $_("settings.more.picFilter"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], false);
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

<Section title={$_("settings.chatSec.title")} lead={$_("settings.chatSec.lead")}>
	{#if s}
		<Group>
			<Row label={$_("settings.chatSec.sendOnEnter")} desc={$_("settings.chatSec.sendOnEnterDesc")} nowrap>
				<Switch checked={s.sendOnEnter} label={$_("settings.chatSec.sendOnEnter")} hideLabel onchange={(v) => appSetting.update({ sendOnEnter: v })} />
			</Row>
		</Group>

		<Group title={$_("settings.chatSec.typingGroup")}>
			<Row label={$_("settings.chatSec.typingStyle")}>
				<div class="typing-row">
					<span class="typing-preview"><TypingIndicator style={typing.style} text={typing.text} image={typing.image} name="Elian" /></span>
					<Select value={typing.style} options={typingStyles} aria-label={$_("settings.chatSec.typingStyle")} onchange={(e) => setTyping({ style: e.currentTarget.value as TypingStyle })} />
				</div>
			</Row>
			{#if typing.style === "image"}
				<Row label={$_("settings.chatSec.typingImage")}>
					<Button size="sm" variant="secondary" onclick={pickTypingImage}>{typing.image ? $_("settings.chatSec.change") : $_("settings.chatSec.choose")}</Button>
					{#if typing.image}<Button size="sm" variant="ghost" onclick={clearTypingImage}>{$_("settings.chatSec.remove")}</Button>{/if}
				</Row>
			{/if}
			<Row label={$_("settings.chatSec.typingText")}>
				<Input value={typing.text} placeholder={"{{char}} ..."} maxlength={60} aria-label={$_("settings.chatSec.typingText")} onchange={(e) => setTyping({ text: e.currentTarget.value })} />
			</Row>
		</Group>

		<Group title={$_("settings.chatSec.notifyGroup")}>
			<Row label={$_("settings.chatSec.notifyEnabled")} desc={$_("settings.chatSec.notifyEnabledDesc")} nowrap>
				<Switch checked={notify.enabled} label={$_("settings.chatSec.notifyEnabled")} hideLabel onchange={(v) => void toggleNotify(v)} />
			</Row>
			{#if notify.enabled}
				<Row label={$_("settings.chatSec.notifyTitle")}>
					<Input value={notify.title} placeholder="Sandcastle · {"{{char}}"}" maxlength={80} aria-label={$_("settings.chatSec.notifyTitle")} onchange={(e) => setNotify({ title: e.currentTarget.value })} />
				</Row>
				{#if !layout.mobile}
					<Row label={$_("settings.chatSec.notifyIcon")}>
						<div class="icon-row">
							{#if notify.icon}<img src={fileSrc(notify.icon)} alt={$_("settings.chatSec.notifyIcon")} />{/if}
							<Button size="sm" onclick={pickIcon}>{$_("settings.chatSec.pickImage")}</Button>
							{#if notify.icon}<Button size="sm" variant="ghost" onclick={() => setNotify({ icon: null })}>{$_("settings.chatSec.resetDefault")}</Button>{/if}
						</div>
					</Row>
				{/if}
				<Row label={$_("settings.chatSec.notifySound")}>
					<Select
						value={notify.soundMode}
						options={notifySoundModes}
						aria-label={$_("settings.chatSec.notifySound")}
						onchange={(e) => setNotify({ soundMode: e.currentTarget.value as NotifySoundMode })}
					/>
				</Row>
				{#if notify.soundMode === "custom"}
					<Row label={$_("settings.chatSec.soundFile")}>
						<div class="sound-row">
							{#if soundFileName}<span class="sound-name" title={soundFileName}>{soundFileName}</span>{/if}
							<Button size="sm" variant="secondary" onclick={pickNotifySound}>{notify.soundPath ? $_("settings.chatSec.change") : $_("settings.chatSec.choose")}</Button>
							{#if notify.soundPath}
								<Button size="sm" variant="ghost" onclick={() => notify.soundPath && playNotifySound(notify.soundPath, notify.soundVolume)}>{$_("settings.chatSec.preview")}</Button>
								<Button size="sm" variant="ghost" onclick={clearNotifySound}>{$_("settings.chatSec.remove")}</Button>
							{/if}
						</div>
					</Row>
					<Row label={$_("settings.chatSec.soundVolume")}>
						<div class="volume-row">
							<input
								type="range"
								min="0"
								max="1"
								step="0.05"
								value={notify.soundVolume ?? 0.8}
								aria-label={$_("settings.chatSec.soundVolume")}
								onchange={(e) => setNotify({ soundVolume: Number(e.currentTarget.value) })}
							/>
							<span class="volume-val">{Math.round((notify.soundVolume ?? 0.8) * 100)}%</span>
						</div>
					</Row>
				{/if}
				<Row label={$_("settings.chatSec.test")}>
					<Button size="sm" variant="secondary" onclick={sendTestNotify}>{$_("settings.chatSec.testNotify")}</Button>
				</Row>
			{/if}
		</Group>

		<Group title={$_("settings.chatSec.defaultsGroup")}>
			<Row label="API" desc={$_("settings.chatSec.apiDesc")}>
				<Select
					value={s.selectedApiProfile ?? ""}
					options={apiOptions}
					aria-label="API"
					onchange={(e) => appSetting.update({ selectedApiProfile: e.currentTarget.value || null })}
				/>
			</Row>
			{#if selectedApi}
				<Row label={$_("settings.chatSec.model")} desc={$_("settings.chatSec.apiDesc")}>
					<ModelPicker
						profile={selectedApi}
						value={s.selectedModel ?? null}
						onchange={(m) => appSetting.update({ selectedModel: m })}
						effort={s.reasoningEffort ?? "auto"}
						oneffort={(e) => appSetting.update({ reasoningEffort: e === "auto" ? null : e })}
						label={$_("settings.chatSec.model")}
					/>
				</Row>
			{/if}
			<Row label={$_("settings.chatSec.preset")} desc={$_("settings.chatSec.presetDesc")}>
				<Select
					value={s.selectedPreset}
					options={presetOptions}
					placeholder={$_("settings.chatSec.noPresets")}
					aria-label={$_("settings.chatSec.preset")}
					onchange={(e) => appSetting.update({ selectedPreset: e.currentTarget.value })}
				/>
			</Row>
			<Row label={$_("settings.chatSec.persona")} desc={$_("settings.chatSec.personaDesc")}>
				<ProfilePicker label={$_("settings.chatSec.persona")} options={personaOptions} value={defaultPersona} onchange={(id) => void setPersona(id)} />
			</Row>
		</Group>

		<Group title={$_("settings.chatSec.multiCharGroup")}>
			<PromptSlotEditor key="multiChar" desc={$_("settings.more.multiCharPromptDesc")} vars={"{{names}} · {{multi_char}}"} />
		</Group>

		{#if plugins.on("status")}
			<Group title={$_("settings.chatSec.varsGroup")}>
				<PromptSlotEditor key="vars" desc={$_("settings.more.varsPromptDesc")} vars={"{{var_list}} · {{vars}}"} />
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
