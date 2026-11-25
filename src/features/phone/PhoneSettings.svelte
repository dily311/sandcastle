<script lang="ts">
	import { _ } from "svelte-i18n";
	// 폰 설정 (설정 > 폰, 폰 화면 오른쪽 패널 정보 탭이 같은 것을 씀)
	// 폰 주인 · 화면 표시 · 배경화면 · 새 채팅방의 기본 API·프리셋 · 폰 모드 프롬프트
	import { onMount } from "svelte";
	import { Button, Input, ProfilePicker, Segmented, Select, Switch, Textarea, type ProfileOption } from "$shared/ui";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import PromptSlotEditor from "$features/settings/PromptSlotEditor.svelte";
	import { chatApiOptions, current, ensureLoaded, orNull, save } from "$features/settings/pluginSettingForm.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { theme } from "$app/theme.svelte";
	import { pickFiles } from "$shared/ipc/dialog";
	import { dropPaths, isImageAvif } from "$app/drop.svelte";
	import { phone, type Who } from "./phone.svelte";

	let phonePrompt = $state("");
	onMount(async () => {
		ensureLoaded();
		await appSetting.load();
		phonePrompt = appSetting.value?.phoneModePrompt ?? "";
	});
	const s = $derived(appSetting.value);
	const plug = $derived(current());
	const presetOptions = $derived([{ value: "", label: $_("phoneExtra.appDefaultPreset") }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	async function savePrompt() {
		await appSetting.update({ phoneModePrompt: phonePrompt });
		toasts.success($_("phoneExtra.savedToast"));
	}

	// 폰 주인: 페르소나(내 폰)와 캐릭터(그 캐릭터의 폰)를 한 목록에서
	const ownerOptions = $derived<ProfileOption[]>([
		...library.personas.map((p) => ({ id: `persona:${p.id}`, name: p.name, avatar: p.avatar, avatarFit: p.avatarFit, color: p.color, description: p.description, group: $_("phoneExtra.ownerPersonaGroup") })),
		...library.characters.map((c) => ({ id: `character:${c.id}`, name: c.name, avatar: c.avatar, avatarFit: c.avatarFit, color: c.color, description: c.description, group: $_("phoneExtra.ownerCharGroup") })),
	]);
	const ownerValue = $derived(phone.prefs.owner ? `${phone.prefs.owner.kind}:${phone.prefs.owner.id}` : null);
	function pickOwner(id: string) {
		const [kind, ...rest] = id.split(":");
		void phone.switchOwner({ kind: kind as Who["kind"], id: rest.join(":") });
	}
	async function addWallpapers(paths: string[]) {
		if (!paths.length) return;
		await theme.addBackgrounds(paths, false);
		const last = theme.setting.backgrounds?.at(-1);
		if (last) phone.setWallpaper(last);
	}
	async function uploadWallpaper() {
		await addWallpapers(await pickFiles([{ name: $_("phoneExtra.imageFileLabel"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true));
	}
	function nowDateStr() {
		const d = new Date();
		return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
	}
	function nowTimeStr() {
		const d = new Date();
		return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
	}
	function toggleCustomClock(v: boolean) {
		const date = plug?.phone.customDate || nowDateStr();
		const time = plug?.phone.customTime || nowTimeStr();
		void save("phone", { customDatetimeEnabled: v, customDate: date, customTime: time });
	}
	function resetClockToNow() {
		void save("phone", { customDate: nowDateStr(), customTime: nowTimeStr() });
	}
	$effect(() => dropPaths({ label: $_("phoneExtra.dropWallLabel"), hint: "PNG·JPG·WEBP·GIF·AVIF", accept: isImageAvif, onpaths: addWallpapers }));
</script>

<div class="phone-settings">
	<section>
		<h4>{$_("phoneExtra.phoneHeading")}</h4>
		<div class="field">
			<span>{$_("phoneExtra.phoneOwnerLabel")}</span>
			<ProfilePicker label={$_("phoneExtra.changeOwnerAria")} options={ownerOptions} value={ownerValue} hint={phone.isCharacterPhone ? $_("phoneExtra.ownerHintChar") : $_("phoneExtra.ownerHintPersona")} onchange={pickOwner} />
		</div>
		<div class="field">
			<span>{$_("phoneExtra.displayLabel")}</span>
			<Segmented
				label={$_("phoneExtra.displayLabel")}
				value={phone.prefs.display}
				options={[
					{ value: "mockup", label: $_("phoneExtra.displayMockup") },
					{ value: "full", label: $_("phoneExtra.displayFull") },
				]}
				onchange={(v) => phone.setDisplay(v)}
			/>
		</div>
		<div class="field">
			<span>{$_("phoneExtra.wallLabel")} <small>{$_("phoneExtra.wallOwnerOnly")}</small></span>
			<BackgroundPicker label={$_("phoneExtra.wallAria")} value={phone.wallpaper} images={theme.setting.backgrounds ?? []} allowNone={false} onpick={(v) => v && phone.setWallpaper(v)} onupload={uploadWallpaper} />
		</div>
	</section>

	{#if plug}
		<section>
			<h4>{$_("phoneExtra.clockHeading")}</h4>
			<div class="row-toggle">
				<span>{$_("phoneExtra.customClockLabel")}</span>
				<Switch checked={!!plug.phone.customDatetimeEnabled} label={$_("phoneExtra.customClockAria")} hideLabel onchange={toggleCustomClock} />
			</div>
			{#if plug.phone.customDatetimeEnabled}
				<div class="dt-grid">
					<label class="field">
						<span>{$_("phoneExtra.dateLabel")}</span>
						<Input type="date" value={plug.phone.customDate || nowDateStr()} aria-label={$_("phoneExtra.dateAria")} onchange={(e) => save("phone", { customDate: e.currentTarget.value })} />
					</label>
					<label class="field">
						<span>{$_("phoneExtra.timeLabel")}</span>
						<Input type="time" value={plug.phone.customTime || nowTimeStr()} aria-label={$_("phoneExtra.timeAria")} onchange={(e) => save("phone", { customTime: e.currentTarget.value })} />
					</label>
				</div>
				<div class="end">
					<Button size="sm" variant="ghost" onclick={resetClockToNow}>{$_("phoneExtra.resetNowBtn")}</Button>
				</div>
			{/if}
			<p class="hint">
				{$_("phoneExtra.clockMacroHint")}
			</p>
		</section>

		<section>
			<h4>{$_("phoneExtra.newRoomDefaultsHeading")}</h4>
			<label class="field"
				><span>API <small>{$_("phoneExtra.apiRoomPriority")}</small></span>
				<Select value={plug.phone.apiProfileId ?? ""} options={chatApiOptions($_("phoneExtra.appDefaultApi"))} onchange={(e) => save("phone", { apiProfileId: orNull(e.currentTarget.value) })} />
			</label>
			{#if plug.phone.apiProfileId}
				<label class="field"
					><span>{$_("phoneExtra.modelLabel")} <small>{$_("phoneExtra.modelDefaultHint")}</small></span>
					<ModelPicker profile={library.apis.find((a) => a.id === plug?.phone.apiProfileId)} value={plug.phone.model || null} onchange={(m) => save("phone", { model: m ?? "" })} label={$_("phoneExtra.modelAria")} />
				</label>
			{/if}
			<label class="field"
				><span>{$_("phoneExtra.presetLabel")} <small>{$_("phoneExtra.presetHint")}</small></span>
				<Select value={plug.phone.presetId ?? ""} options={presetOptions} onchange={(e) => save("phone", { presetId: orNull(e.currentTarget.value) })} />
			</label>
		</section>
	{/if}

	{#if s}
		<section>
			<h4>{$_("phoneExtra.modePromptHeading")}</h4>
			<p class="hint">{$_("phoneExtra.modePromptHint")}</p>
			<Textarea bind:value={phonePrompt} autosize maxRows={14} rows={5} mono aria-label={$_("phoneExtra.modePromptHeading")} />
			<div class="end">
				<Button size="sm" variant="ghost" disabled={phonePrompt === s.phoneModePrompt} onclick={() => (phonePrompt = s.phoneModePrompt)}>{$_("phoneExtra.revertBtn")}</Button>
				<Button size="sm" variant="primary" disabled={phonePrompt === s.phoneModePrompt} onclick={savePrompt}>{$_("common.save")}</Button>
			</div>
		</section>
	{/if}

	<section>
		<h4>{$_("phoneExtra.profilePromptHeading")}</h4>
		<PromptSlotEditor key="phoneProfile" desc={$_("phoneExtra.profilePromptDesc")} vars={$_("phoneExtra.charNameVar")} />
	</section>

	<section>
		<h4>{$_("phoneExtra.snsPromptHeading")}</h4>
		<PromptSlotEditor key="phoneSns" desc={$_("phoneExtra.snsPromptDesc")} vars={$_("phoneExtra.charNameVar")} />
	</section>
</div>

<style>
	.phone-settings {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	h4 {
		margin: 0;
		font-size: var(--text-sm);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.field small,
	.hint {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.hint {
		margin: 0;
	}
	.end {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.row-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.dt-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
	}
</style>
