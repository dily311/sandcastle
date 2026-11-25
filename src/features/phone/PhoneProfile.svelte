<script lang="ts">
	import {
		ArrowDown,
		Heart,
		Image as ImageIcon,
		MessageCircle,
		Music,
		Pause,
		Pencil,
		Play,
		Settings2,
		Sparkles,
		Volume2,
		VolumeX,
		X,
	} from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { Avatar, Button, Input, Spinner, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { toasts } from "$shared/stores/toast.svelte";
	import type { AvatarFit } from "$ipc/bindings";
	import { phone, type PhoneProfileData, type Who } from "./phone.svelte";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { gradientCss, gradientOf } from "$shared/ui/backgrounds";
	import { theme } from "$app/theme.svelte";

	export interface ProfileTarget {
		key: string;
		name: string;
		avatar: string | null;
		avatarFit?: AvatarFit | null;
		color?: string | null;
		who?: Who;
		/** 1:1 채팅·AI 생성 시 연결할 대화 상대 (다인봇 등장인물 등 who와 다를 때) */
		chatWho?: Who;
	}

	let {
		target,
		convId,
		onback,
		onchat,
	}: {
		target: ProfileTarget;
		convId?: string;
		onback: () => void;
		onchat?: () => void;
	} = $props();

	const DEFAULT_COVER =
		"linear-gradient(165deg, #3c3a44 0%, #5c5358 38%, #342f36 72%, #1d1b20 100%)";

	const contactStatus = $derived(
		target.who ? (phone.contacts.find((c) => c.contactKind === target.who?.kind && c.contactId === target.who?.id)?.status ?? "") : "",
	);

	const stored = $derived(phone.getProfile(target.key));
	const profile = $derived<PhoneProfileData>(
		stored ?? {
			handle: target.name.toLowerCase().replace(/\s+/g, "_") || "user",
			status: contactStatus,
			bio: "",
			link: "",
			followers: "0",
			following: "0",
			posts: [],
			coverImage: null,
			note: "",
			bgmTitle: "",
			bgmArtist: "",
			bgmLiked: false,
			updatedAt: 0,
		},
	);

	let generating = $state(false);
	let editing = $state(false);
	let muted = $state(true);
	let playing = $state(false);

	let editStatus = $state("");
	let editNote = $state("");
	let editBgmTitle = $state("");
	let editBgmArtist = $state("");
	let editCover = $state<string | null>(null);

	const displayStatus = $derived(editing ? editStatus : profile.status || contactStatus);
	const activeCover = $derived(editing ? editCover : (profile.coverImage ?? null));
	const coverBg = $derived.by(() => {
		if (!activeCover) return DEFAULT_COVER;
		const g = gradientOf(activeCover);
		if (g) return gradientCss(g);
		const src = fileSrc(activeCover);
		if (src) return `center / cover no-repeat url("${src.replace(/"/g, "%22")}")`;
		return DEFAULT_COVER;
	});

	function persist(next: PhoneProfileData) {
		phone.saveProfile(target.key, next);
		if (target.who && next.status !== contactStatus) {
			void phone.setStatus(target.who, next.status);
		}
	}

	function openEdit() {
		editStatus = displayStatus;
		editNote = profile.note ?? "";
		editBgmTitle = profile.bgmTitle ?? "";
		editBgmArtist = profile.bgmArtist ?? "";
		editCover = profile.coverImage ?? null;
		editing = true;
	}

	function saveEdit() {
		persist({
			...profile,
			status: editStatus.trim(),
			note: editNote.trim(),
			bgmTitle: editBgmTitle.trim(),
			bgmArtist: editBgmArtist.trim(),
			coverImage: editCover,
			updatedAt: Date.now(),
		});
		editing = false;
		toasts.success($_("phoneProfileUi.savedToast"));
	}
	async function uploadBackgrounds() {
		const paths = await pickFiles([{ name: $_("phoneProfileUi.imageFilterName"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true);
		if (!paths.length) return;
		await theme.addBackgrounds(paths, false);
		const last = theme.setting.backgrounds?.at(-1) ?? null;
		if (last) setPickCoverImage(last);
	}

	async function pickCoverImage() {
		const [path] = await pickFiles([IMAGE_FILES], false);
		if (!path) return;
		const imported = await call(commands.importGalleryImages([path]), { source: "phone" });
		const savedPath = imported[0]?.path;
		if (!savedPath) return;
		setPickCoverImage(savedPath);
	}

	function setPickCoverImage(path: string | null) {
		if (editing) {
			editCover = path;
		} else {
			persist({ ...profile, coverImage: path, updatedAt: Date.now() });
			toasts.success(path ? $_("phoneProfileUi.bgChangedToast") : $_("phoneProfileUi.bgResetToast"));
		}
	}

	function toggleBgmLike() {
		persist({ ...profile, bgmLiked: !profile.bgmLiked, updatedAt: Date.now() });
	}

	function parseAiJson(raw: string): Record<string, unknown> | null {
		const cleaned = raw
			.trim()
			.replace(/^```(?:json)?\s*/i, "")
			.replace(/\s*```$/i, "")
			.trim();
		const start = cleaned.indexOf("{");
		const end = cleaned.lastIndexOf("}");
		if (start < 0 || end <= start) return null;
		try {
			return JSON.parse(cleaned.slice(start, end + 1));
		} catch {
			return null;
		}
	}

	async function resolveProfilePrompt(charName: string): Promise<string> {
		const fallback = [
			`지금까지의 대화와 캐릭터 설정을 바탕으로 '{{char}}'의 카카오톡 스타일 메신저 프로필 꾸미기 정보를 JSON으로만 생성해줘.`,
			`설명이나 코드블록 없이 순수 JSON 객체 하나만 출력해.`,
			`형식:`,
			`{`,
			`  "status": "이름 아래에 표시될 짧은 상태 메시지 (15자 내외)",`,
			`  "note": "배경 화면 위에 띄울 짧은 감성 메모 문구 (1~2줄, \\n으로 줄바꿈 가능)",`,
			`  "bgmTitle": "캐릭터 분위기나 현재 기분에 어울리는 프로필 뮤직 제목",`,
			`  "bgmArtist": "프로필 뮤직 아티스트명"`,
			`}`,
		].join("\n");
		try {
			const [ps, defs] = await Promise.all([
				call(commands.getPluginSetting(), { source: "phone", silent: true }).catch(() => null),
				call(commands.defaultAppPrompts(), { source: "phone", silent: true }).catch(() => null),
			]);
			const slot = ps?.prompts?.phoneProfile;
			const chosen = slot?.items.find((v) => v.id === slot.selected)?.text.trim();
			const tpl = chosen || defs?.phoneProfile?.items[0]?.text || fallback;
			return tpl.replaceAll("{{char}}", charName);
		} catch {
			return fallback.replaceAll("{{char}}", charName);
		}
	}

	async function generateWithAi() {
		if (generating) return;
		generating = true;
		try {
			const prompt = await resolveProfilePrompt(target.name);
			let cid = convId;
			if (!cid) {
				const chatWho = target.chatWho ?? target.who;
				if (chatWho) cid = (await phone.open(chatWho))?.id;
			}
			if (!cid) {
				toasts.error($_("phoneProfileUi.openChatFirstErr"));
				return;
			}
			const raw = await call(commands.phoneGenerateProfile(cid, prompt), { source: "phone" });
			const parsed = parseAiJson(raw);
			if (!parsed) {
				toasts.error($_("phoneProfileUi.aiParseErr"));
				return;
			}
			const next: PhoneProfileData = {
				...profile,
				status: typeof parsed.status === "string" ? parsed.status.trim() : profile.status,
				note: typeof parsed.note === "string" ? parsed.note.trim() : (profile.note ?? ""),
				bgmTitle: typeof parsed.bgmTitle === "string" ? parsed.bgmTitle.trim() : (profile.bgmTitle ?? ""),
				bgmArtist: typeof parsed.bgmArtist === "string" ? parsed.bgmArtist.trim() : (profile.bgmArtist ?? ""),
				updatedAt: Date.now(),
			};
			persist(next);
			toasts.success($_("phoneProfileUi.aiDecoratedToast"));
		} finally {
			generating = false;
		}
	}
</script>

<div class="kk-profile" style:background={coverBg}>
	<div class="kk-scrim" aria-hidden="true"></div>

	<!-- 상단 바 -->
	<header class="kk-top">
		<div class="kk-top-left">
			<button type="button" class="kk-plain" aria-label={$_("common.close")} onclick={onback}><X /></button>
			{#if profile.bgmTitle}
				<button
					type="button"
					class="kk-mute"
					aria-label={muted ? $_("phoneProfileUi.unmuteAria") : $_("phoneProfileUi.muteAria")}
					onclick={() => (muted = !muted)}
				>
					{#if muted}<VolumeX />{:else}<Volume2 />{/if}
				</button>
			{/if}
		</div>
		<div class="kk-top-right">
			<button type="button" class="kk-circle" title={$_("phoneProfileUi.aiDecorateTitle")} disabled={generating} onclick={generateWithAi}>
				{#if generating}<Spinner size={15} />{:else}<Sparkles />{/if}
			</button>
			<button type="button" class="kk-circle" title={$_("phoneProfileUi.changeBgTitle")} onclick={pickCoverImage}>
				<ImageIcon />
			</button>
			<button type="button" class="kk-circle" title={$_("phoneProfileUi.editProfileTitle")} onclick={openEdit}>
				<Settings2 />
			</button>
		</div>
	</header>

	<!-- 상단 위젯 영역: 프로필 뮤직 / 감성 메모 -->
	<div class="kk-widgets">
		{#if profile.bgmTitle}
			<div class="kk-bgm">
				<div class="kk-bgm-thumb">
					{#if target.avatar}
						<img src={fileSrc(target.avatar)} alt={$_("phoneProfileUi.albumCoverAlt")} />
					{:else}
						<Music />
					{/if}
				</div>
				<div class="kk-bgm-info">
					<b>{profile.bgmTitle}</b>
					<span>{profile.bgmArtist || target.name}</span>
				</div>
				<button type="button" class="kk-bgm-btn" class:liked={profile.bgmLiked} aria-label={$_("phoneProfileUi.likeAria")} onclick={toggleBgmLike}>
					<Heart />
				</button>
				<button
					type="button"
					class="kk-bgm-btn"
					aria-label={playing ? $_("phoneProfileUi.pauseAria") : $_("phoneProfileUi.playAria")}
					onclick={() => {
						playing = !playing;
						if (playing) muted = false;
					}}
				>
					{#if playing}<Pause />{:else}<Play />{/if}
				</button>
			</div>
		{/if}

		{#if profile.note}
			<button type="button" class="kk-note" title={$_("phoneProfileUi.editNoteTitle")} onclick={openEdit}>
				<span class="kk-note-hearts" aria-hidden="true"><Heart /><Heart /></span>
				<span class="kk-note-text">{profile.note}</span>
				<span class="kk-note-arrow" aria-hidden="true"><ArrowDown /></span>
			</button>
		{/if}
	</div>

	<!-- 하단 중앙 아바타 + 이름 + 상태 메시지 -->
	<section class="kk-center">
		<div class="kk-avatar">
			<Avatar name={target.name} src={fileSrc(target.avatar)} fit={target.avatarFit} color={target.color} size={86} />
		</div>
		<h2 class="kk-name">{target.name}</h2>
		<button type="button" class="kk-status" title={$_("phoneProfileUi.editStatusTitle")} onclick={openEdit}>
			{displayStatus || $_("phoneProfileUi.emptyStatusPrompt")}
		</button>
	</section>

	<!-- 하단 3버튼 바 -->
	<footer class="kk-actions">
		<button type="button" class="kk-act" onclick={() => (onchat ? onchat() : onback())}>
			<MessageCircle />
			<span>{$_("phoneProfileUi.oneOnOneChat")}</span>
		</button>
		<button type="button" class="kk-act" onclick={openEdit}>
			<Pencil />
			<span>{$_("phoneProfileUi.editProfileTitle")}</span>
		</button>
		<button type="button" class="kk-act" disabled={generating} onclick={generateWithAi}>
			{#if generating}<Spinner size={18} />{:else}<Sparkles />{/if}
			<span>{generating ? $_("phoneProfileUi.decorating") : $_("phoneProfileUi.aiDecorateBtn")}</span>
		</button>
	</footer>

	<!-- 편집 시트 -->
	{#if editing}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div class="kk-sheet-backdrop" onclick={() => (editing = false)}></div>
		<div class="kk-sheet" role="dialog" aria-label={$_("phoneProfileUi.editProfileTitle")}>
			<div class="kk-sheet-head">
				<strong>{$_("phoneProfileUi.editProfileTitle")}</strong>
				<button type="button" class="kk-sheet-close" aria-label={$_("common.close")} onclick={() => (editing = false)}><X /></button>
			</div>
			<div class="kk-sheet-body">
				<label class="kk-field">
					<span>{$_("phoneProfileUi.statusLabel")}</span>
					<Input bind:value={editStatus} placeholder={$_("phoneProfileUi.statusPlaceholder")} />
				</label>
				<label class="kk-field">
					<span>{$_("phoneProfileUi.noteLabel")}</span>
					<Textarea bind:value={editNote} rows={2} autosize placeholder={$_("phoneProfileUi.notePlaceholder")} />
				</label>
				<div class="kk-grid-2">
					<label class="kk-field">
						<span>{$_("phoneProfileUi.bgmTitleLabel")}</span>
						<Input bind:value={editBgmTitle} placeholder={$_("phoneProfileUi.bgmTitlePlaceholder")} />
					</label>
					<label class="kk-field">
						<span>{$_("phoneProfileUi.bgmArtistLabel")}</span>
						<Input bind:value={editBgmArtist} placeholder={$_("phoneProfileUi.bgmArtistPlaceholder")} />
					</label>
				</div>
				<div class="kk-cover-row">
					<span>{$_("phoneProfileUi.bgImageLabel")}</span>
					<div class="kk-cover-btns">
						<BackgroundPicker label={$_("phoneProfileUi.bgImageLabel")} value={editCover} images={theme.setting.backgrounds ?? []} onpick={(v) => setPickCoverImage(v)} onupload={uploadBackgrounds} />

						<Button size="sm" variant="ghost" onclick={pickCoverImage}>{#snippet icon()}<ImageIcon />{/snippet}{$_("phoneProfileUi.pickPhotoBtn")}</Button>
						{#if editCover}
							<Button size="sm" variant="ghost" onclick={() => (editCover = null)}>{$_("phoneProfileUi.defaultBgBtn")}</Button>
						{/if}
					</div>
				</div>
			</div>
			<div class="kk-sheet-foot">
				<Button size="sm" variant="ghost" onclick={() => (editing = false)}>{$_("common.cancel")}</Button>
				<Button size="sm" variant="primary" onclick={saveEdit}>{$_("common.save")}</Button>
			</div>
		</div>
	{/if}
</div>

<style>
	.kk-profile {
		position: relative;
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		color: #fff;
		overflow: hidden;
		user-select: none;
		
	}
	.kk-scrim {
		position: absolute;
		inset: 0;
		background: linear-gradient(
			180deg,
			rgb(0 0 0 / 0.42) 0%,
			rgb(0 0 0 / 0.14) 32%,
			rgb(0 0 0 / 0.22) 60%,
			rgb(0 0 0 / 0.68) 100%
		);
		pointer-events: none;
	}
	.kk-top,
	.kk-widgets,
	.kk-center,
	.kk-actions {
		position: relative;
		z-index: 1;
	}
	.kk-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px 8px;
	}
	.kk-top-left,
	.kk-top-right {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.kk-plain {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: none;
		background: none;
		color: #fff;
		cursor: pointer;
	}
	.kk-plain :global(svg) {
		width: 22px;
		height: 22px;
	}
	.kk-mute {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: rgb(255 255 255 / 0.24);
		color: #fff;
		cursor: pointer;
		backdrop-filter: blur(6px);
	}
	.kk-mute :global(svg) {
		width: 15px;
		height: 15px;
	}
	.kk-circle {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 1px solid rgb(255 255 255 / 0.6);
		border-radius: 50%;
		background: rgb(0 0 0 / 0.18);
		color: #fff;
		cursor: pointer;
		backdrop-filter: blur(4px);
	}
	.kk-circle:disabled {
		opacity: 0.55;
		cursor: default;
	}
	.kk-circle :global(svg) {
		width: 15px;
		height: 15px;
	}
	.kk-widgets {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 8px 16px;
		gap: 18px;
		min-height: 0;
	}
	.kk-bgm {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 6px;
		border-top: 1px solid rgb(255 255 255 / 0.26);
		border-bottom: 1px solid rgb(255 255 255 / 0.26);
		background: rgb(0 0 0 / 0.12);
		backdrop-filter: blur(4px);
	}
	.kk-bgm-thumb {
		width: 36px;
		height: 36px;
		border-radius: 6px;
		overflow: hidden;
		background: rgb(255 255 255 / 0.2);
		display: grid;
		place-items: center;
		flex: none;
	}
	.kk-bgm-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.kk-bgm-thumb :global(svg) {
		width: 16px;
		height: 16px;
	}
	.kk-bgm-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.kk-bgm-info b {
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.kk-bgm-info span {
		font-size: 11px;
		color: rgb(255 255 255 / 0.78);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.kk-bgm-btn {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		background: none;
		color: #fff;
		cursor: pointer;
	}
	.kk-bgm-btn :global(svg) {
		width: 17px;
		height: 17px;
	}
	.kk-bgm-btn.liked {
		color: #ff6b81;
	}
	.kk-bgm-btn.liked :global(svg) {
		fill: #ff6b81;
	}
	.kk-note {
		margin-top: 8px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border: none;
		background: none;
		color: #fff;
		font: inherit;
		text-align: center;
		cursor: pointer;
		text-shadow: 0 1px 6px rgb(0 0 0 / 0.55);
	}
	.kk-note-hearts {
		display: inline-flex;
		gap: 2px;
		color: #f9a8d4;
	}
	.kk-note-hearts :global(svg) {
		width: 15px;
		height: 15px;
		fill: rgb(249 168 212 / 0.35);
	}
	.kk-note-text {
		font-size: 13.5px;
		font-weight: 500;
		line-height: 1.45;
		white-space: pre-wrap;
		max-width: 230px;
	}
	.kk-note-arrow {
		opacity: 0.9;
	}
	.kk-note-arrow :global(svg) {
		width: 18px;
		height: 18px;
	}
	.kk-center {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 0 20px 18px;
		text-align: center;
	}
	.kk-avatar :global(.avatar) {
		border-radius: 38% !important;
		box-shadow: 0 6px 20px rgb(0 0 0 / 0.35);
	}
	.kk-name {
		margin: 4px 0 0;
		font-size: 18px;
		font-weight: 700;
		letter-spacing: -0.01em;
		text-shadow: 0 1px 6px rgb(0 0 0 / 0.45);
	}
	.kk-status {
		padding: 2px 8px;
		border: none;
		background: none;
		color: rgb(255 255 255 / 0.88);
		font: inherit;
		font-size: 13px;
		line-height: 1.4;
		cursor: pointer;
		text-shadow: 0 1px 4px rgb(0 0 0 / 0.45);
		max-width: 100%;
	}
	.kk-status:hover {
		color: #fff;
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.kk-actions {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		border-top: 1px solid rgb(255 255 255 / 0.22);
		padding: 14px 8px 24px;
		background: rgb(0 0 0 / 0.16);
		backdrop-filter: blur(4px);
	}
	.kk-act {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 4px 0;
		border: none;
		background: none;
		color: #fff;
		font: inherit;
		font-size: 11.5px;
		cursor: pointer;
	}
	.kk-act:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.kk-act :global(svg) {
		width: 20px;
		height: 20px;
	}
	.kk-sheet-backdrop {
		position: absolute;
		inset: 0;
		z-index: 5;
		background: rgb(0 0 0 / 0.45);
	}
	.kk-sheet {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 6;
		display: flex;
		flex-direction: column;
		max-height: 82%;
		border-radius: 18px 18px 0 0;
		background: var(--surface);
		color: var(--text);
		border-top: 1px solid var(--border);
		box-shadow: 0 -8px 28px rgb(0 0 0 / 0.35);
	}
	.kk-sheet-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 16px 10px;
		border-bottom: 1px solid var(--border);
	}
	.kk-sheet-head strong {
		font-size: 15px;
	}
	.kk-sheet-close {
		display: grid;
		place-items: center;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 0;
	}
	.kk-sheet-close :global(svg) {
		width: 18px;
		height: 18px;
	}
	.kk-sheet-body {
		padding: 12px 16px;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.kk-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.kk-field span,
	.kk-cover-row > span {
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-muted);
	}
	.kk-grid-2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.kk-cover-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding-top: 2px;
	}
	.kk-cover-btns {
		display: flex;
		gap: 6px;
	}
	.kk-sheet-foot {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		padding: 10px 16px 18px;
		border-top: 1px solid var(--border);
	}
</style>
