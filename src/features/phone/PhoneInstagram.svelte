<script lang="ts">
	import {
		ChevronLeft,
		Grid3x3,
		Heart,
		ImagePlus,
		Link2,
		MapPin,
		MessageCircle,
		Pencil,
		Plus,
		Rows3,
		Send,
		Sparkles,
		Trash2,
		X,
	} from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { Avatar, Button, Input, Spinner, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { toasts } from "$shared/stores/toast.svelte";
	import type { AvatarFit } from "$ipc/bindings";
	import { phone, type PhonePost, type PhoneProfileData, type Who } from "./phone.svelte";

	export interface ProfileTarget {
		key: string;
		name: string;
		avatar: string | null;
		avatarFit?: AvatarFit | null;
		color?: string | null;
		who?: Who;
	}

	let {
		target,
		convId,
		onback,
	}: {
		target: ProfileTarget;
		convId: string;
		onback: () => void;
	} = $props();

	const CARD_MOODS = [
		"linear-gradient(135deg, #fce7f3 0%, #e0e7ff 100%)",
		"linear-gradient(135deg, #fef3c7 0%, #fde68a 100%)",
		"linear-gradient(135deg, #dbeafe 0%, #e0f2fe 100%)",
		"linear-gradient(135deg, #dcfce7 0%, #f0fdf4 100%)",
		"linear-gradient(135deg, #f3e8ff 0%, #fae8ff 100%)",
		"linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 100%)",
	];

	function defaultHandle(name: string) {
		const ascii = name
			.toLowerCase()
			.replace(/[^a-z0-9._]/g, "_")
			.replace(/_+/g, "_")
			.replace(/^_|_$/g, "");
		return ascii || name.replace(/\s+/g, "_").slice(0, 16) || "user";
	}

	const contactStatus = $derived(
		target.who ? (phone.contacts.find((c) => c.contactKind === target.who?.kind && c.contactId === target.who?.id)?.status ?? "") : "",
	);

	const stored = $derived(phone.getProfile(target.key));
	const profile = $derived<PhoneProfileData>(
		stored ?? {
			handle: defaultHandle(target.name),
			status: contactStatus,
			bio: "",
			link: "",
			followers: "0",
			following: "0",
			posts: [],
			updatedAt: 0,
		},
	);

	let viewMode = $state<"grid" | "feed">("grid");
	let generating = $state(false);
	let editingProfile = $state(false);
	let editHandle = $state("");
	let editStatus = $state("");
	let editBio = $state("");
	let editLink = $state("");
	let editFollowers = $state("");
	let editFollowing = $state("");

	let selectedPostId = $state<string | null>(null);
	const selectedPost = $derived(selectedPostId ? (profile.posts.find((p) => p.id === selectedPostId) ?? null) : null);

	let editingPostId = $state<string | null>(null);
	let postCaption = $state("");
	let postLocation = $state("");
	let postDate = $state("");
	let postLikes = $state("0");
	let postImage = $state<string | null>(null);
	let creatingPost = $state(false);
	let commentDraft = $state<Record<string, string>>({});

	function uid() {
		return `${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
	}

	function persist(next: PhoneProfileData) {
		phone.saveProfile(target.key, next);
		if (target.who && next.status !== contactStatus) {
			void phone.setStatus(target.who, next.status);
		}
	}

	function startEditProfile() {
		editHandle = profile.handle;
		editStatus = profile.status || contactStatus;
		editBio = profile.bio;
		editLink = profile.link;
		editFollowers = profile.followers;
		editFollowing = profile.following;
		editingProfile = true;
	}

	function saveProfileEdit() {
		const next: PhoneProfileData = {
			...profile,
			handle: editHandle.trim().replace(/^@+/, "") || defaultHandle(target.name),
			status: editStatus.trim(),
			bio: editBio.trim(),
			link: editLink.trim(),
			followers: editFollowers.trim() || "0",
			following: editFollowing.trim() || "0",
			updatedAt: Date.now(),
		};
		persist(next);
		editingProfile = false;
		toasts.success($_("phoneProfileUi.snsSavedToast"));
	}

	function parseAiJson(raw: string): (Omit<Partial<PhoneProfileData>, "posts"> & { posts?: Array<Record<string, unknown>> }) | null {
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

	async function resolveSnsPrompt(charName: string): Promise<string> {
		const fallback = [
			`지금까지의 대화와 캐릭터 설정을 바탕으로 '{{char}}'의 인스타그램 스타일 개인 SNS 페이지를 JSON으로만 생성해줘.`,
			`설명이나 코드블록 없이 순수 JSON 객체 하나만 출력해.`,
			`형식:`,
			`{`,
			`  "handle": "영문소문자_아이디",`,
			`  "status": "인스타 노트용 짧은 상태 메시지 (20자 이내)",`,
			`  "bio": "캐릭터 성격이 드러나는 자기소개 (1~3줄, \\n으로 줄바꿈)",`,
			`  "link": "위치 또는 짧은 링크 문구 (선택, 없으면 빈 문자열)",`,
			`  "followers": "팔로워 수 문자열 (예: 128 또는 1.2만)",`,
			`  "following": "팔로잉 수 문자열 (예: 42)",`,
			`  "posts": [`,
			`    {`,
			`      "caption": "캐릭터 말투의 최근 SNS 게시물 글 (해시태그 포함 가능)",`,
			`      "location": "장소 (선택)",`,
			`      "date": "상대 시간 (예: 2시간 전, 어제, 3일 전)",`,
			`      "likes": 24,`,
			`      "comments": [{ "author": "댓글 작성자 이름", "text": "짧은 댓글" }]`,
			`    }`,
			`  ]`,
			`}`,
			`posts는 3~4개 작성해줘.`,
		].join("\n");
		try {
			const [ps, defs] = await Promise.all([
				call(commands.getPluginSetting(), { source: "phone", silent: true }).catch(() => null),
				call(commands.defaultAppPrompts(), { source: "phone", silent: true }).catch(() => null),
			]);
			const slot = ps?.prompts?.phoneSns;
			const chosen = slot?.items.find((v) => v.id === slot.selected)?.text.trim();
			const tpl = chosen || defs?.phoneSns?.items[0]?.text || fallback;
			return tpl.replaceAll("{{char}}", charName);
		} catch {
			return fallback.replaceAll("{{char}}", charName);
		}
	}

	async function generateWithAi() {
		if (generating) return;
		generating = true;
		try {
			const prompt = await resolveSnsPrompt(target.name);
			const raw = await call(commands.phoneGenerateProfile(convId, prompt), { source: "phone" });
			const parsed = parseAiJson(raw);
			if (!parsed) {
				toasts.error($_("phoneProfileUi.snsAiParseErr"));
				return;
			}
			const existingImages = profile.posts.map((p) => p.image).filter(Boolean);
			const rawPosts = Array.isArray(parsed.posts) ? parsed.posts : [];
			const posts: PhonePost[] = rawPosts.map((p, i) => ({
				id: uid(),
				caption: typeof p.caption === "string" ? p.caption : "",
				location: typeof p.location === "string" ? p.location : "",
				date: typeof p.date === "string" && p.date.trim() ? p.date : $_("phoneProfileUi.snsJustNow"),
				likes: typeof p.likes === "number" && Number.isFinite(p.likes) ? Math.max(0, Math.round(p.likes)) : 12 + i * 7,
				liked: false,
				image: existingImages[i] ?? null,
				mood: CARD_MOODS[i % CARD_MOODS.length],
				comments: Array.isArray(p.comments)
					? p.comments
							.filter((c): c is { author?: unknown; text?: unknown } => !!c && typeof c === "object")
							.map((c) => ({
								id: uid(),
								author: typeof c.author === "string" && c.author.trim() ? c.author : $_("phoneProfileUi.snsFriendFallback"),
								text: typeof c.text === "string" ? c.text : "",
							}))
							.filter((c) => c.text.trim())
					: [],
			}));
			const next: PhoneProfileData = {
				handle: typeof parsed.handle === "string" && parsed.handle.trim() ? parsed.handle.trim().replace(/^@+/, "") : profile.handle,
				status: typeof parsed.status === "string" ? parsed.status.trim() : profile.status,
				bio: typeof parsed.bio === "string" ? parsed.bio.trim() : profile.bio,
				link: typeof parsed.link === "string" ? parsed.link.trim() : profile.link,
				followers: parsed.followers != null ? String(parsed.followers) : profile.followers,
				following: parsed.following != null ? String(parsed.following) : profile.following,
				posts: posts.length ? posts : profile.posts,
				updatedAt: Date.now(),
			};
			persist(next);
			toasts.success($_("phoneProfileUi.snsAiGeneratedToast"));
		} finally {
			generating = false;
		}
	}

	function toggleLike(postId: string) {
		const nextPosts = profile.posts.map((p) =>
			p.id === postId ? { ...p, liked: !p.liked, likes: Math.max(0, p.likes + (p.liked ? -1 : 1)) } : p,
		);
		persist({ ...profile, posts: nextPosts });
	}

	function startNewPost() {
		creatingPost = true;
		editingPostId = null;
		postCaption = "";
		postLocation = "";
		postDate = $_("phoneProfileUi.snsJustNow");
		postLikes = "0";
		postImage = null;
	}

	function startEditPost(p: PhonePost) {
		creatingPost = false;
		editingPostId = p.id;
		postCaption = p.caption;
		postLocation = p.location ?? "";
		postDate = p.date;
		postLikes = String(p.likes);
		postImage = p.image ?? null;
	}

	async function pickPostPhoto() {
		const [path] = await pickFiles([IMAGE_FILES], false);
		if (!path) return;
		const imported = await call(commands.importGalleryImages([path]), { source: "phone" });
		if (imported[0]?.path) postImage = imported[0].path;
	}

	function savePost() {
		if (!postCaption.trim() && !postImage) return;
		const likesNum = Math.max(0, Number.parseInt(postLikes, 10) || 0);
		if (creatingPost) {
			const created: PhonePost = {
				id: uid(),
				caption: postCaption.trim(),
				location: postLocation.trim(),
				date: postDate.trim() || $_("phoneProfileUi.snsJustNow"),
				likes: likesNum,
				liked: false,
				image: postImage,
				mood: CARD_MOODS[profile.posts.length % CARD_MOODS.length],
				comments: [],
			};
			persist({ ...profile, posts: [created, ...profile.posts] });
			creatingPost = false;
			toasts.success($_("phoneProfileUi.snsPostAddedToast"));
		} else if (editingPostId) {
			const nextPosts = profile.posts.map((p) =>
				p.id === editingPostId
					? {
							...p,
							caption: postCaption.trim(),
							location: postLocation.trim(),
							date: postDate.trim() || p.date,
							likes: likesNum,
							image: postImage,
						}
					: p,
			);
			persist({ ...profile, posts: nextPosts });
			editingPostId = null;
			toasts.success($_("phoneProfileUi.snsPostUpdatedToast"));
		}
	}

	function deletePost(postId: string) {
		const nextPosts = profile.posts.filter((p) => p.id !== postId);
		persist({ ...profile, posts: nextPosts });
		if (selectedPostId === postId) selectedPostId = null;
		if (editingPostId === postId) editingPostId = null;
		toasts.info($_("phoneProfileUi.snsPostDeletedToast"));
	}

	function addComment(postId: string) {
		const text = (commentDraft[postId] ?? "").trim();
		if (!text) return;
		const author = phone.owner?.name ?? $_("phoneProfileUi.snsMeFallback");
		const nextPosts = profile.posts.map((p) =>
			p.id === postId ? { ...p, comments: [...p.comments, { id: uid(), author, text }] } : p,
		);
		persist({ ...profile, posts: nextPosts });
		commentDraft[postId] = "";
	}

	function deleteComment(postId: string, commentId: string) {
		const nextPosts = profile.posts.map((p) =>
			p.id === postId ? { ...p, comments: p.comments.filter((c) => c.id !== commentId) } : p,
		);
		persist({ ...profile, posts: nextPosts });
	}
</script>

<div class="ig-page">
	<div class="ig-bar">
		<button
			type="button"
			class="plain"
			aria-label={$_("phoneProfileUi.snsBackAria")}
			onclick={() => {
				if (selectedPostId) selectedPostId = null;
				else onback();
			}}
		>
			<ChevronLeft />
		</button>
		<strong class="ig-handle">{selectedPost ? $_("phoneProfileUi.snsPostHeading") : `@${profile.handle}`}</strong>
		<div class="ig-bar-actions">
			{#if !selectedPost}
				<button type="button" class="plain" title={$_("phoneProfileUi.snsAiFillTitle")} disabled={generating} onclick={generateWithAi}>
					{#if generating}<Spinner size={16} />{:else}<Sparkles />{/if}
				</button>
				<button
					type="button"
					class="plain"
					class:active={editingProfile}
					title={$_("phoneProfileUi.snsEditPageTitle")}
					onclick={() => (editingProfile ? (editingProfile = false) : startEditProfile())}
				>
					<Pencil />
				</button>
			{/if}
		</div>
	</div>

	<div class="ig-scroll">
		{#if selectedPost}
			{@const p = selectedPost}
			<article class="ig-post">
				<div class="post-head">
					<Avatar name={target.name} src={fileSrc(target.avatar)} fit={target.avatarFit} color={target.color} size={32} />
					<div class="post-author">
						<b>{profile.handle}</b>
						{#if p.location}<span><MapPin /> {p.location}</span>{/if}
					</div>
					<div class="post-tools">
						<button type="button" class="plain sm" title={$_("phoneProfileUi.snsEditPostTitle")} onclick={() => startEditPost(p)}><Pencil /></button>
						<button type="button" class="plain sm danger" title={$_("phoneProfileUi.snsDeletePostTitle")} onclick={() => deletePost(p.id)}><Trash2 /></button>
					</div>
				</div>

				{#if editingPostId === p.id}
					<div class="edit-box">
						<div class="edit-photo-row">
							{#if postImage}
								<img class="edit-thumb" src={fileSrc(postImage)} alt={$_("phoneProfileUi.snsPostImgAlt")} />
								<Button size="sm" variant="ghost" onclick={() => (postImage = null)}>{$_("phoneProfileUi.snsRemovePhotoBtn")}</Button>
							{/if}
							<Button size="sm" variant="ghost" onclick={pickPostPhoto}>{#snippet icon()}<ImagePlus />{/snippet}{$_("phoneProfileUi.pickPhotoBtn")}</Button>
						</div>
						<Textarea bind:value={postCaption} rows={3} autosize placeholder={$_("phoneProfileUi.snsPostCaptionPlaceholder")} />
						<div class="edit-grid-2">
							<Input bind:value={postLocation} placeholder={$_("phoneProfileUi.snsLocationPlaceholder")} />
							<Input bind:value={postDate} placeholder={$_("phoneProfileUi.snsDatePlaceholder")} />
						</div>
						<Input bind:value={postLikes} type="number" placeholder={$_("phoneProfileUi.snsLikesPlaceholder")} />
						<div class="edit-actions">
							<Button size="sm" variant="ghost" onclick={() => (editingPostId = null)}>{$_("common.cancel")}</Button>
							<Button size="sm" variant="primary" onclick={savePost}>{$_("common.save")}</Button>
						</div>
					</div>
				{:else}
					{#if p.image}
						<img class="post-img" src={fileSrc(p.image)} alt={p.caption || $_("phoneProfileUi.snsPostPhotoAlt")} />
					{:else}
						<div class="post-card-media" style:background={p.mood || CARD_MOODS[0]}>
							<p>{p.caption}</p>
						</div>
					{/if}
					<div class="post-body">
						<div class="post-actions">
							<button type="button" class="like-btn" class:liked={p.liked} onclick={() => toggleLike(p.id)}>
								<Heart />
								<span>{p.likes}</span>
							</button>
							<span class="cmt-count"><MessageCircle /> {p.comments.length}</span>
							<span class="post-date">{p.date}</span>
						</div>
						{#if p.caption}
							<p class="post-caption"><b>{profile.handle}</b> {p.caption}</p>
						{/if}
						{#if p.comments.length}
							<ul class="post-comments">
								{#each p.comments as c (c.id)}
									<li>
										<span><b>{c.author}</b> {c.text}</span>
										<button type="button" class="cmt-del" aria-label={$_("phoneProfileUi.snsDeleteCommentAria")} onclick={() => deleteComment(p.id, c.id)}><X /></button>
									</li>
								{/each}
							</ul>
						{/if}
						<form
							class="cmt-form"
							onsubmit={(e) => {
								e.preventDefault();
								addComment(p.id);
							}}
						>
							<input bind:value={commentDraft[p.id]} placeholder={$_("phoneProfileUi.snsCommentPlaceholder")} aria-label={$_("phoneProfileUi.snsCommentInputAria")} />
							<button type="submit" disabled={!(commentDraft[p.id] ?? "").trim()} aria-label={$_("phoneProfileUi.snsCommentSubmitAria")}><Send /></button>
						</form>
					</div>
				{/if}
			</article>
		{:else}
			<!-- Instagram Profile Header -->
			<section class="ig-header">
				<div class="ig-top-row">
					<div class="ig-av-wrap">
						{#if profile.status || contactStatus}
							<button type="button" class="ig-note" title={$_("phoneProfileUi.editStatusTitle")} onclick={startEditProfile}>
								{profile.status || contactStatus}
							</button>
						{/if}
						<div class="ig-ring">
							<Avatar name={target.name} src={fileSrc(target.avatar)} fit={target.avatarFit} color={target.color} size={74} />
						</div>
					</div>
					<div class="ig-stats">
						<div><b>{profile.posts.length}</b><span>{$_("phoneProfileUi.snsStatPosts")}</span></div>
						<div><b>{profile.followers}</b><span>{$_("phoneProfileUi.snsStatFollowers")}</span></div>
						<div><b>{profile.following}</b><span>{$_("phoneProfileUi.snsStatFollowing")}</span></div>
					</div>
				</div>

				<div class="ig-bio">
					<strong class="ig-name">{target.name}</strong>
					{#if profile.bio}
						<p class="ig-bio-text">{profile.bio}</p>
					{:else if !editingProfile}
						<p class="ig-bio-empty">{$_("phoneProfileUi.snsBioEmpty")}</p>
					{/if}
					{#if profile.link}
						<p class="ig-link"><Link2 /> {profile.link}</p>
					{/if}
				</div>

				<div class="ig-btns">
					<button type="button" class="ig-btn primary" onclick={onback}>{$_("phoneProfileUi.snsMessageBtn")}</button>
					<button type="button" class="ig-btn" onclick={() => (editingProfile ? (editingProfile = false) : startEditProfile())}>
						{editingProfile ? $_("phoneProfileUi.snsCloseEditBtn") : $_("phoneProfileUi.snsEditPageBtn")}
					</button>
					<button type="button" class="ig-btn ai" disabled={generating} onclick={generateWithAi}>
						<Sparkles />
						<span>{generating ? $_("phoneProfileUi.snsGenerating") : profile.updatedAt ? $_("phoneProfileUi.snsAiRefreshBtn") : $_("phoneProfileUi.snsAiFillBtn")}</span>
					</button>
				</div>

				{#if editingProfile}
					<div class="edit-box">
						<div class="edit-grid-2">
							<label class="ef"><span>{$_("phoneProfileUi.snsHandleLabel")}</span><Input bind:value={editHandle} placeholder="handle" /></label>
							<label class="ef"><span>{$_("phoneProfileUi.snsStatusNoteLabel")}</span><Input bind:value={editStatus} placeholder={$_("phoneProfileUi.snsStatusNotePlaceholder")} /></label>
						</div>
						<label class="ef"><span>{$_("phoneProfileUi.snsBioLabel")}</span><Textarea bind:value={editBio} rows={3} autosize placeholder={$_("phoneProfileUi.snsBioPlaceholder")} /></label>
						<label class="ef"><span>{$_("phoneProfileUi.snsLinkLabel")}</span><Input bind:value={editLink} placeholder={$_("phoneProfileUi.snsLinkPlaceholder")} /></label>
						<div class="edit-grid-2">
							<label class="ef"><span>{$_("phoneProfileUi.snsStatFollowers")}</span><Input bind:value={editFollowers} placeholder="128" /></label>
							<label class="ef"><span>{$_("phoneProfileUi.snsStatFollowing")}</span><Input bind:value={editFollowing} placeholder="42" /></label>
						</div>
						<div class="edit-actions">
							<Button size="sm" variant="ghost" onclick={() => (editingProfile = false)}>{$_("common.cancel")}</Button>
							<Button size="sm" variant="primary" onclick={saveProfileEdit}>{$_("common.save")}</Button>
						</div>
					</div>
				{/if}
			</section>

			<!-- Tabs -->
			<div class="ig-tabs">
				<button type="button" class:on={viewMode === "grid"} aria-label={$_("phoneProfileUi.snsGridViewAria")} onclick={() => (viewMode = "grid")}>
					<Grid3x3 />
				</button>
				<button type="button" class:on={viewMode === "feed"} aria-label={$_("phoneProfileUi.snsFeedViewAria")} onclick={() => (viewMode = "feed")}>
					<Rows3 />
				</button>
				<button type="button" class:on={creatingPost} aria-label={$_("phoneProfileUi.snsNewPostAria")} title={$_("phoneProfileUi.snsNewPostAria")} onclick={startNewPost}>
					<Plus />
				</button>
			</div>

			{#if creatingPost}
				<div class="edit-box new-post">
					<div class="edit-photo-row">
						{#if postImage}
							<img class="edit-thumb" src={fileSrc(postImage)} alt={$_("phoneProfileUi.snsNewPostImgAlt")} />
							<Button size="sm" variant="ghost" onclick={() => (postImage = null)}>{$_("phoneProfileUi.snsRemoveBtn")}</Button>
						{/if}
						<Button size="sm" variant="ghost" onclick={pickPostPhoto}>{#snippet icon()}<ImagePlus />{/snippet}{$_("phoneProfileUi.snsAttachPhotoBtn")}</Button>
					</div>
					<Textarea bind:value={postCaption} rows={3} autosize placeholder={$_("phoneProfileUi.snsNewPostPlaceholder")} />
					<div class="edit-grid-2">
						<Input bind:value={postLocation} placeholder={$_("phoneProfileUi.snsLocationPlaceholder")} />
						<Input bind:value={postDate} placeholder={$_("phoneProfileUi.snsNewPostDatePlaceholder")} />
					</div>
					<div class="edit-actions">
						<Button size="sm" variant="ghost" onclick={() => (creatingPost = false)}>{$_("common.cancel")}</Button>
						<Button size="sm" variant="primary" disabled={!postCaption.trim() && !postImage} onclick={savePost}>{$_("phoneProfileUi.snsPublishBtn")}</Button>
					</div>
				</div>
			{/if}

			{#if !profile.posts.length}
				<div class="ig-empty">
					<p>{$_("phoneProfileUi.snsEmptyPosts")}</p>
					<button type="button" class="ig-btn ai" disabled={generating} onclick={generateWithAi}>
						<Sparkles />
						<span>{generating ? $_("phoneProfileUi.snsGenerating") : $_("phoneProfileUi.snsGenerateEmptyBtn")}</span>
					</button>
				</div>
			{:else if viewMode === "grid"}
				<div class="ig-grid">
					{#each profile.posts as p, i (p.id)}
						<button type="button" class="ig-tile" onclick={() => (selectedPostId = p.id)}>
							{#if p.image}
								<img src={fileSrc(p.image)} alt={p.caption || $_("phoneProfileUi.snsPostHeading")} loading="lazy" />
							{:else}
								<div class="ig-tile-text" style:background={p.mood || CARD_MOODS[i % CARD_MOODS.length]}>
									<span>{p.caption}</span>
								</div>
							{/if}
							<span class="ig-tile-meta"><Heart /> {p.likes}</span>
						</button>
					{/each}
				</div>
			{:else}
				<div class="ig-feed">
					{#each profile.posts as p, i (p.id)}
						<article class="ig-post">
							<div class="post-head">
								<Avatar name={target.name} src={fileSrc(target.avatar)} fit={target.avatarFit} color={target.color} size={30} />
								<div class="post-author">
									<b>{profile.handle}</b>
									{#if p.location}<span><MapPin /> {p.location}</span>{/if}
								</div>
								<div class="post-tools">
									<button type="button" class="plain sm" title={$_("phoneProfileUi.snsEditPostTitle")} onclick={() => ((selectedPostId = p.id), startEditPost(p))}><Pencil /></button>
									<button type="button" class="plain sm danger" title={$_("phoneProfileUi.snsDeletePostTitle")} onclick={() => deletePost(p.id)}><Trash2 /></button>
								</div>
							</div>
							{#if p.image}
								<img class="post-img" src={fileSrc(p.image)} alt={p.caption || $_("phoneProfileUi.snsPostPhotoAlt")} />
							{:else}
								<div class="post-card-media" style:background={p.mood || CARD_MOODS[i % CARD_MOODS.length]}>
									<p>{p.caption}</p>
								</div>
							{/if}
							<div class="post-body">
								<div class="post-actions">
									<button type="button" class="like-btn" class:liked={p.liked} onclick={() => toggleLike(p.id)}>
										<Heart />
										<span>{p.likes}</span>
									</button>
									<button type="button" class="cmt-count btn" onclick={() => (selectedPostId = p.id)}>
										<MessageCircle /> {p.comments.length}
									</button>
									<span class="post-date">{p.date}</span>
								</div>
								{#if p.caption}
									<p class="post-caption"><b>{profile.handle}</b> {p.caption}</p>
								{/if}
							</div>
						</article>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.ig-page {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: #fff;
		color: #191919;
	}
	.ig-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 14px 10px;
		border-bottom: 1px solid #ececef;
	}
	.ig-handle {
		flex: 1;
		font-size: 16px;
		font-weight: 700;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ig-bar-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.plain {
		display: grid;
		place-items: center;
		padding: 0;
		border: none;
		background: none;
		color: #222;
		cursor: pointer;
	}
	.plain :global(svg) {
		width: 20px;
		height: 20px;
	}
	.plain.sm :global(svg) {
		width: 16px;
		height: 16px;
	}
	.plain.active {
		color: var(--accent);
	}
	.plain.danger {
		color: #d94343;
	}
	.ig-scroll {
		flex: 1;
		overflow: auto;
		min-height: 0;
	}
	.ig-header {
		padding: 14px 16px 12px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.ig-top-row {
		display: flex;
		align-items: center;
		gap: 18px;
	}
	.ig-av-wrap {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		flex: none;
	}
	.ig-note {
		position: relative;
		max-width: 112px;
		margin-bottom: 6px;
		padding: 4px 8px;
		border: 1px solid #e6e6ea;
		border-radius: 12px;
		background: #fff;
		box-shadow: 0 2px 8px rgb(0 0 0 / 0.08);
		color: #222;
		font: inherit;
		font-size: 11px;
		line-height: 1.3;
		text-align: center;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ig-ring {
		padding: 3px;
		border-radius: 50%;
		background: linear-gradient(45deg, #f09433 0%, #e6683c 25%, #dc2743 50%, #cc2366 75%, #bc1888 100%);
	}
	.ig-ring :global(.avatar) {
		border: 2px solid #fff;
		border-radius: 50% !important;
	}
	.ig-stats {
		flex: 1;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		text-align: center;
		gap: 4px;
	}
	.ig-stats div {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1px;
	}
	.ig-stats b {
		font-size: 15px;
		font-weight: 700;
	}
	.ig-stats span {
		font-size: 11.5px;
		color: #666;
	}
	.ig-bio {
		display: flex;
		flex-direction: column;
		gap: 3px;
		font-size: 13px;
	}
	.ig-name {
		font-size: 14px;
		font-weight: 700;
	}
	.ig-bio-text {
		margin: 0;
		white-space: pre-wrap;
		line-height: 1.45;
		color: #262626;
	}
	.ig-bio-empty {
		margin: 0;
		color: #888;
		font-size: 12px;
	}
	.ig-link {
		margin: 2px 0 0;
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: #00376b;
		font-weight: 500;
	}
	.ig-link :global(svg) {
		width: 13px;
		height: 13px;
	}
	.ig-btns {
		display: grid;
		grid-template-columns: 1fr 1fr auto;
		gap: 6px;
	}
	.ig-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		height: 32px;
		padding: 0 10px;
		border: none;
		border-radius: 8px;
		background: #efefef;
		color: #191919;
		font: inherit;
		font-size: 12.5px;
		font-weight: 600;
		cursor: pointer;
	}
	.ig-btn:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.ig-btn.primary {
		background: var(--accent);
		color: #fff;
	}
	.ig-btn.ai {
		background: color-mix(in oklab, var(--accent) 14%, #f4f4f8);
		color: var(--accent);
	}
	.ig-btn :global(svg) {
		width: 14px;
		height: 14px;
	}
	.ig-tabs {
		display: grid;
		grid-template-columns: 1fr 1fr 48px;
		border-top: 1px solid #ececef;
		border-bottom: 1px solid #ececef;
	}
	.ig-tabs button {
		display: grid;
		place-items: center;
		height: 40px;
		border: none;
		border-bottom: 2px solid transparent;
		background: none;
		color: #8a8a8a;
		cursor: pointer;
	}
	.ig-tabs button.on {
		color: #191919;
		border-bottom-color: #191919;
	}
	.ig-tabs :global(svg) {
		width: 19px;
		height: 19px;
	}
	.ig-empty {
		padding: 36px 20px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		color: #888;
		font-size: 13px;
	}
	.ig-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 2px;
	}
	.ig-tile {
		position: relative;
		aspect-ratio: 1;
		padding: 0;
		border: none;
		background: #f2f2f5;
		overflow: hidden;
		cursor: pointer;
		text-align: left;
	}
	.ig-tile img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.ig-tile-text {
		width: 100%;
		height: 100%;
		padding: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: #262626;
		font-size: 11.5px;
		line-height: 1.4;
		text-align: center;
	}
	.ig-tile-text span {
		display: -webkit-box;
		-webkit-line-clamp: 4;
		line-clamp: 4;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.ig-tile-meta {
		position: absolute;
		right: 6px;
		bottom: 5px;
		display: inline-flex;
		align-items: center;
		gap: 3px;
		padding: 2px 6px;
		border-radius: 10px;
		background: rgb(0 0 0 / 0.45);
		color: #fff;
		font-size: 10px;
		font-weight: 600;
	}
	.ig-tile-meta :global(svg) {
		width: 10px;
		height: 10px;
	}
	.ig-feed {
		display: flex;
		flex-direction: column;
	}
	.ig-post {
		border-bottom: 1px solid #ececef;
	}
	.post-head {
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 10px 14px;
	}
	.post-author {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.post-author b {
		font-size: 13px;
	}
	.post-author span {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		font-size: 11px;
		color: #666;
	}
	.post-author span :global(svg) {
		width: 11px;
		height: 11px;
	}
	.post-tools {
		display: flex;
		gap: 8px;
	}
	.post-img {
		width: 100%;
		max-height: 360px;
		object-fit: cover;
		display: block;
		background: #f5f5f5;
	}
	.post-card-media {
		min-height: 190px;
		padding: 24px 20px;
		display: grid;
		place-items: center;
		text-align: center;
	}
	.post-card-media p {
		margin: 0;
		font-size: 14px;
		line-height: 1.65;
		color: #222;
		white-space: pre-wrap;
	}
	.post-body {
		padding: 10px 14px 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.post-actions {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.like-btn,
	.cmt-count {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 0;
		border: none;
		background: none;
		color: #262626;
		font: inherit;
		font-size: 13px;
		font-weight: 600;
	}
	.like-btn,
	.cmt-count.btn {
		cursor: pointer;
	}
	.like-btn.liked {
		color: #ed4956;
	}
	.like-btn.liked :global(svg) {
		fill: #ed4956;
	}
	.like-btn :global(svg),
	.cmt-count :global(svg) {
		width: 18px;
		height: 18px;
	}
	.post-date {
		margin-left: auto;
		font-size: 11px;
		color: #888;
	}
	.post-caption {
		margin: 0;
		font-size: 13px;
		line-height: 1.45;
		white-space: pre-wrap;
	}
	.post-comments {
		list-style: none;
		margin: 4px 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.post-comments li {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 6px;
		font-size: 12.5px;
		color: #333;
	}
	.cmt-del {
		border: none;
		background: none;
		color: #aaa;
		cursor: pointer;
		padding: 0;
		display: grid;
		place-items: center;
	}
	.cmt-del :global(svg) {
		width: 13px;
		height: 13px;
	}
	.cmt-form {
		display: flex;
		gap: 6px;
		margin-top: 4px;
		padding-top: 6px;
		border-top: 1px solid #f0f0f3;
	}
	.cmt-form input {
		flex: 1;
		border: none;
		background: #f5f5f7;
		border-radius: 14px;
		padding: 6px 10px;
		font: inherit;
		font-size: 12.5px;
		color: #191919;
		outline: none;
	}
	.cmt-form button {
		border: none;
		background: none;
		color: var(--accent);
		cursor: pointer;
		padding: 0 4px;
		display: grid;
		place-items: center;
	}
	.cmt-form button:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.cmt-form button :global(svg) {
		width: 16px;
		height: 16px;
	}
	.edit-box {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px;
		border-radius: 10px;
		background: #f7f7fa;
		border: 1px solid #e6e6ec;
		--surface: #fff;
		--text: #191919;
		--text-muted: #666;
		--border: #dcdce2;
	}
	.edit-box.new-post {
		margin: 10px 12px;
	}
	.edit-grid-2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.ef {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.ef span {
		font-size: 11px;
		font-weight: 600;
		color: #555;
	}
	.edit-photo-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.edit-thumb {
		width: 48px;
		height: 48px;
		object-fit: cover;
		border-radius: 8px;
	}
	.edit-actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}
</style>
