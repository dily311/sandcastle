<script lang="ts">
	import { _ } from "svelte-i18n";
	// 폰 화면의 오른쪽 패널
	// - 대화방이면 대화 화면과 같은 공통 탭(대화 설정 · 컨텍스트·프리셋 · 분기)을 ChatPanel로 그대로
	// - '정보' 탭: 상대 카드와 폰 설정
	import { ExternalLink } from "@lucide/svelte";
	import { Avatar, Button, type ProfileOption } from "$shared/ui";
	import { panel } from "$app/panel.svelte";
	import ChatPanel from "$features/character/ChatPanel.svelte";
	import type { ChatBookmark } from "$features/character/bookmarks";
	import { metaOf, type ChatSession } from "$features/character/session.svelte";
	import { fileSrc } from "$shared/ipc/file";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { lookup, phone, type Who } from "./phone.svelte";
	import PhoneSettings from "./PhoneSettings.svelte";

	let {
		room,
		session,
		onopen,
		onjump,
		oncard,
		draft,
	}: {
		room: Conversation | null;
		session: ChatSession | null;
		onopen: (id: string, messageId?: string) => void;
		onjump?: (messageId: string, mark?: ChatBookmark) => void;
		oncard?: (mark: ChatBookmark) => void;
		draft?: { get: () => string; set: (v: string) => void };
	} = $props();
	const shared = new Set(["settings", "context", "toc", "bookmarks", "memory", "lint", "branches", "assets", "quickreply", "status"]);
	const chapters = $derived.by(() => {
		const out: Record<string, number> = {};
		let n = 0;
		for (const m of session?.messages ?? []) if (metaOf(m).chapter) out[m.id] = ++n;
		return out;
	});

	// 방은 열 때의 사본일 수 있으니 목록의 최신 대화를 씀 (meta 저장이 바로 보이게)
	const conv = $derived(room ? (library.conversation(room.id) ?? room) : null);
	const contact = $derived(conv ? lookup(phone.contactOf(conv)) : null);
	const status = $derived(
		contact ? (phone.contacts.find((c) => c.contactKind === contact.kind && c.contactId === contact.id)?.status ?? "") : "",
	);

	function openPage(w: Who) {
		void nav.go(w.kind === "persona" ? { page: "persona", personaId: w.id } : { page: "character", charId: w.id });
	}
	const kindLabel = (k: string) => (k === "persona" ? $_("phoneExtra.kindPersona") : $_("phoneExtra.kindChar"));

	// 폰에서의 "나" = 폰 주인. 페르소나(내 폰)와 캐릭터(그 캐릭터의 폰)를 한 목록에서 고른다
	const ownerOptions = $derived<ProfileOption[]>([
		...library.personas.map((p) => ({ id: `persona:${p.id}`, name: p.name, avatar: p.avatar, avatarFit: p.avatarFit, color: p.color, description: p.description, group: $_("phoneExtra.ownerPersonaGroup") })),
		...library.characters.map((c) => ({ id: `character:${c.id}`, name: c.name, avatar: c.avatar, avatarFit: c.avatarFit, color: c.color, description: c.description, group: $_("phoneExtra.ownerCharGroup") })),
	]);
	const ownerValue = $derived(phone.prefs.owner ? `${phone.prefs.owner.kind}:${phone.prefs.owner.id}` : null);
	function pickOwner(id: string) {
		const [kind, ...rest] = id.split(":");
		void phone.switchOwner({ kind: kind as Who["kind"], id: rest.join(":") });
	}
	const me = $derived({
		label: $_("phoneExtra.meOwnerLabel"),
		options: ownerOptions,
		value: ownerValue,
		hint: phone.isCharacterPhone ? $_("phoneExtra.ownerHintChar") : $_("phoneExtra.ownerHintPersona"),
		onchange: pickOwner,
	});
</script>

{#if conv && session && shared.has(panel.tab)}
	<ChatPanel {conv} {session} {chapters} onjump={onjump ?? (() => {})} {oncard} {onopen} {draft} />
{:else if shared.has(panel.tab)}
	<div class="panel-body">
		<p class="hint">{$_("phoneExtra.openRoomHint", { values: { what: panel.tab === "context" ? $_("phoneExtra.whatContext") : $_("phoneExtra.whatSettings") } })}</p>
		<p class="hint">{$_("phoneExtra.defaultApiHint")}</p>
	</div>
{:else}
<div class="panel-body">
		{#if conv && contact}
			<section>
				<h4>{$_("phoneExtra.partnerHeading")}</h4>
				<div class="card">
					<Avatar name={contact.name} src={fileSrc(contact.avatar)} fit={contact.avatarFit} color={contact.color} size={56} />
					<div class="who">
						<b>{contact.name}</b>
						<small>{kindLabel(contact.kind)}{status ? ` · ${status}` : ""}</small>
					</div>
					<Button size="sm" variant="ghost" onclick={() => openPage(contact)}>{#snippet icon()}<ExternalLink />{/snippet}{$_("phoneExtra.openBtn")}</Button>
				</div>
			</section>

		{/if}

		<!-- 설정 > 폰과 같은 설정 -->
		<PhoneSettings />
</div>
{/if}

<style>
	.card {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.who {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		line-height: 1.35;
		font-size: var(--text-sm);
	}
	.who small {
		color: var(--text-muted);
		font-size: var(--text-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
