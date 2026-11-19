<script lang="ts">
	// 폰 설정 (설정 > 폰, 폰 화면 오른쪽 패널 정보 탭이 같은 것을 씀)
	// 폰 주인 · 화면 표시 · 배경화면 · 새 채팅방의 기본 API·프리셋 · 폰 모드 프롬프트
	import { onMount } from "svelte";
	import { Button, ProfilePicker, Segmented, Select, Textarea, type ProfileOption } from "$shared/ui";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
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
	const presetOptions = $derived([{ value: "", label: "앱 기본 프리셋" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	async function savePrompt() {
		await appSetting.update({ phoneModePrompt: phonePrompt });
		toasts.success("저장함");
	}

	// 폰 주인: 페르소나(내 폰)와 캐릭터(그 캐릭터의 폰)를 한 목록에서
	const ownerOptions = $derived<ProfileOption[]>([
		...library.personas.map((p) => ({ id: `persona:${p.id}`, name: p.name, avatar: p.avatar, color: p.color, description: p.description, group: "내 페르소나의 폰" })),
		...library.characters.map((c) => ({ id: `character:${c.id}`, name: c.name, avatar: c.avatar, color: c.color, description: c.description, group: "캐릭터의 폰 (그 캐릭터가 되거나 엿봄)" })),
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
		await addWallpapers(await pickFiles([{ name: "그림", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true));
	}
	$effect(() => dropPaths({ label: "폰 배경화면으로 올리기", hint: "PNG·JPG·WEBP·GIF·AVIF", accept: isImageAvif, onpaths: addWallpapers }));
</script>

<div class="phone-settings">
	<section>
		<h4>폰</h4>
		<div class="field">
			<span>폰 주인</span>
			<ProfilePicker label="폰 주인 바꾸기" options={ownerOptions} value={ownerValue} hint={phone.isCharacterPhone ? "캐릭터 폰 · 바꾸면 그 폰으로 넘어감" : "바꾸면 그 사람의 폰으로 넘어감"} onchange={pickOwner} />
		</div>
		<div class="field">
			<span>화면 표시</span>
			<Segmented
				label="화면 표시"
				value={phone.prefs.display}
				options={[
					{ value: "mockup", label: "기기 모양" },
					{ value: "full", label: "화면 가득" },
				]}
				onchange={(v) => phone.setDisplay(v)}
			/>
		</div>
		<div class="field">
			<span>배경화면 <small>이 폰 주인의 폰에만</small></span>
			<BackgroundPicker label="폰 배경화면" value={phone.wallpaper} images={theme.setting.backgrounds ?? []} allowNone={false} onpick={(v) => v && phone.setWallpaper(v)} onupload={uploadWallpaper} />
		</div>
	</section>

	{#if plug}
		<section>
			<h4>새 채팅방 기본값</h4>
			<label class="field"
				><span>API <small>채팅방 설정에서 고른 것이 먼저</small></span>
				<Select value={plug.phone.apiProfileId ?? ""} options={chatApiOptions("앱 기본 API")} onchange={(e) => save("phone", { apiProfileId: orNull(e.currentTarget.value) })} />
			</label>
			{#if plug.phone.apiProfileId}
				<label class="field"
					><span>모델 <small>비우면 그 API의 기본 모델</small></span>
					<ModelPicker profile={library.apis.find((a) => a.id === plug?.phone.apiProfileId)} value={plug.phone.model || null} onchange={(m) => save("phone", { model: m ?? "" })} label="폰 기본 모델" />
				</label>
			{/if}
			<label class="field"
				><span>프리셋 <small>짧은 메신저 답장용 프리셋을 따로 두면 좋음</small></span>
				<Select value={plug.phone.presetId ?? ""} options={presetOptions} onchange={(e) => save("phone", { presetId: orNull(e.currentTarget.value) })} />
			</label>
		</section>
	{/if}

	{#if s}
		<section>
			<h4>폰 모드 프롬프트</h4>
			<p class="hint">{"폰 대화에만 붙는 시스템 프롬프트. {{char}}, {{user}} 사용 가능. 비우면 붙이지 않음"}</p>
			<Textarea bind:value={phonePrompt} autosize maxRows={14} rows={5} mono aria-label="폰 모드 프롬프트" />
			<div class="end">
				<Button size="sm" variant="ghost" disabled={phonePrompt === s.phoneModePrompt} onclick={() => (phonePrompt = s.phoneModePrompt)}>되돌리기</Button>
				<Button size="sm" variant="primary" disabled={phonePrompt === s.phoneModePrompt} onclick={savePrompt}>저장</Button>
			</div>
		</section>
	{/if}
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
</style>
