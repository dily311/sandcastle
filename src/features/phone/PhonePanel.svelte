<script lang="ts">
	// 폰 화면의 오른쪽 패널: (대화방이면) 작가 노트·메모, 상대, 이 대화 / 폰
	import { ExternalLink, FileJson, FileText } from "@lucide/svelte";
	import { Avatar, Button, Segmented, Select } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { convActions } from "$features/character/actions";
	import NotesPanel from "$features/character/NotesPanel.svelte";
	import { lookup, phone, WALLPAPERS, type Who } from "./phone.svelte";

	let { room }: { room: Conversation | null } = $props();

	// 방은 열 때의 사본일 수 있으니 목록의 최신 대화를 씀 (meta 저장이 바로 보이게)
	const conv = $derived(room ? (library.conversation(room.id) ?? room) : null);
	const contact = $derived(conv ? lookup(phone.contactOf(conv)) : null);
	const status = $derived(
		contact ? (phone.contacts.find((c) => c.contactKind === contact.kind && c.contactId === contact.id)?.status ?? "") : "",
	);
	const owner = $derived(phone.owner);

	const DEFAULT = "";
	const apiOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const presetOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	function setOption(p: { apiProfileId?: string; promptPresetId?: string }) {
		if (!conv) return;
		void library.setOptions(conv.id, {
			personaId: conv.personaId,
			promptPresetId: (p.promptPresetId ?? conv.promptPresetId ?? "") || null,
			apiProfileId: (p.apiProfileId ?? conv.apiProfileId ?? "") || null,
		});
	}
	function openPage(w: Who) {
		void nav.go(w.kind === "persona" ? { page: "persona", personaId: w.id } : { page: "character", charId: w.id });
	}
	const kindLabel = (k: string) => (k === "persona" ? "페르소나" : "캐릭터");
</script>

<div class="panel-body">
		{#if conv && contact}
			<NotesPanel {conv} bare />
			<section>
				<h4>상대</h4>
				<div class="card">
					<Avatar name={contact.name} src={fileSrc(contact.avatar)} color={contact.color} size={56} />
					<div class="who">
						<b>{contact.name}</b>
						<small>{kindLabel(contact.kind)}{status ? ` · ${status}` : ""}</small>
					</div>
					<Button size="sm" variant="ghost" onclick={() => openPage(contact)}>{#snippet icon()}<ExternalLink />{/snippet}열기</Button>
				</div>
				{#if contact.description}<p class="desc">{contact.description}</p>{/if}
			</section>

			<section>
				<h4>이 대화 <span class="faint">메시지 {conv.messageCount}개</span></h4>
				<label class="row"><span>API</span><Select value={conv.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} /></label>
				<label class="row"><span>프리셋</span><Select value={conv.promptPresetId ?? DEFAULT} options={presetOptions} onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} /></label>
				<div class="two">
					<Button size="sm" onclick={() => convActions.exportAs(conv, "jsonl")}>{#snippet icon()}<FileJson />{/snippet}.jsonl</Button>
					<Button size="sm" onclick={() => convActions.exportAs(conv, "txt")}>{#snippet icon()}<FileText />{/snippet}.txt</Button>
				</div>
			</section>
		{/if}

		{#if owner}
			<section>
				<h4>폰</h4>
				<button class="card owner" onclick={() => openPage(owner)}>
					<Avatar name={owner.name} src={fileSrc(owner.avatar)} color={owner.color} size={40} />
					<span class="who"><b>{owner.name}의 폰</b><small>{phone.isCharacterPhone ? "캐릭터 폰 · 그 캐릭터가 되거나 엿봄" : "내 페르소나 폰"}</small></span>
				</button>
				<Segmented
					label="화면 표시"
					value={phone.prefs.display}
					options={[
						{ value: "mockup", label: "기기 모양" },
						{ value: "full", label: "화면 가득" },
					]}
					onchange={(v) => phone.setDisplay(v)}
				/>
				<div class="walls" role="radiogroup" aria-label="배경화면">
					{#each Object.entries(WALLPAPERS) as [id, bg] (id)}
						<button class="wall" role="radio" aria-checked={phone.wallpaper === id} class:on={phone.wallpaper === id} style:background={bg} aria-label="배경 {id}" onclick={() => phone.setWallpaper(id)}></button>
					{/each}
				</div>
				<p class="hint">주인 바꾸기·연락처는 폰 안의 설정·연락처 앱에서</p>
			</section>
		{/if}
</div>

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
	.card.owner {
		width: 100%;
		text-align: left;
		cursor: pointer;
	}
	.card.owner:hover {
		background: var(--surface-hover);
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
	.desc {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
		line-height: 1.55;
		display: -webkit-box;
		-webkit-line-clamp: 6;
		line-clamp: 6;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.row {
		display: grid;
		grid-template-columns: 64px minmax(0, 1fr);
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
	}
	.walls {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(36px, 1fr));
		gap: var(--space-2);
	}
	.wall {
		aspect-ratio: 9 / 16;
		border: 2px solid transparent;
		border-radius: var(--radius-sm);
		cursor: pointer;
		box-shadow: 0 0 0 1px var(--border);
	}
	.wall.on {
		border-color: var(--accent);
	}
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
