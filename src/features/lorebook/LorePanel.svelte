<script lang="ts">
	// 대화 패널 '로어북' 탭: 이 대화에 걸리는 책(캐릭터·페르소나·대화), 대화에만 붙이기, 항목 빠르게 찾아 고치기
	import { ExternalLink, Search } from "@lucide/svelte";
	import { IconButton, Input, Switch } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import EntryItem from "./EntryItem.svelte";
	import { entryTitle, lorebook } from "./lorebook.svelte";

	let { conv }: { conv: Conversation } = $props();

	$effect(() => {
		if (!lorebook.loaded) void lorebook.load();
	});

	const character = $derived(library.character(conv.characterId));
	const persona = $derived(library.persona(conv.personaId) ?? library.personas[0]);
	const fromChar = $derived(new Set(character?.worldInfoIds ?? []));
	const fromPersona = $derived(new Set(persona?.worldInfoIds ?? []));
	const fromConv = $derived(new Set(conv.worldInfoIds));
	const active = $derived(lorebook.books.filter((b) => fromChar.has(b.id) || fromPersona.has(b.id) || fromConv.has(b.id)));
	// 걸린 책 먼저, 그다음 나머지
	const rows = $derived([...active, ...lorebook.books.filter((b) => !active.includes(b))]);

	async function toggle(bookId: string, on: boolean) {
		const next = on ? [...conv.worldInfoIds, bookId] : conv.worldInfoIds.filter((x) => x !== bookId);
		await library.setWorldInfo(conv.id, [...new Set(next)]);
	}

	// ---- 걸린 책들의 항목 찾기 ----
	$effect(() => {
		for (const b of active) void lorebook.loadEntries(b.id);
	});
	let query = $state("");
	const all = $derived(active.flatMap((b) => (lorebook.entries[b.id] ?? []).map((e) => ({ e, book: b.name }))));
	const hits = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return [];
		return all.filter(({ e }) => entryTitle(e).toLowerCase().includes(q) || e.keys.some((k) => k.toLowerCase().includes(q)) || e.content.toLowerCase().includes(q)).slice(0, 30);
	});
	let openId = $state<string | null>(null);
</script>

<div class="panel-body">
	<section>
		<h4>로어북 <span class="faint">걸린 책 {active.length}권 · 항목 {all.filter((x) => x.e.enabled).length}개 켜짐</span></h4>
		{#if lorebook.books.length}
			<ul class="books">
				{#each rows as b (b.id)}
					{@const src = [fromChar.has(b.id) && "캐릭터", fromPersona.has(b.id) && "페르소나"].filter(Boolean).join("·")}
					<li class:on={active.includes(b)}>
						<Switch
							checked={fromConv.has(b.id) || !!src}
							disabled={!!src && !fromConv.has(b.id)}
							label="{b.name}을 이 대화에 붙이기"
							hideLabel
							onchange={(v) => toggle(b.id, v)}
						/>
						<span class="t">
							<b>{b.name}</b>
							<small>{src ? `${src}에서 씀` : fromConv.has(b.id) ? "이 대화에만 붙임" : "안 씀"} · 항목 {lorebook.counts[b.id]?.entries ?? 0}개</small>
						</span>
						<IconButton label="{b.name} 로어북 화면에서 열기" size="sm" onclick={() => nav.go({ page: "lorebook", bookId: b.id })}><ExternalLink /></IconButton>
					</li>
				{/each}
			</ul>
			<p class="hint">캐릭터·페르소나에 붙은 책은 각 편집 화면에서 빼야 꺼짐. 여기 스위치는 이 대화에만 붙임</p>
		{:else}
			<p class="panel-none">로어북 없음. <button class="link" onclick={() => nav.go({ page: "lorebook" })}>로어북 화면</button>에서 만들거나 가져옴</p>
		{/if}
	</section>

	{#if active.length}
		<section>
			<h4>항목 찾아서 고치기</h4>
			<Input bind:value={query} placeholder="제목·키워드·내용" aria-label="로어북 항목 검색" type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			{#if query.trim()}
				{#if hits.length}
					<ul class="entries">
						{#each hits as h (h.e.id)}
							<EntryItem entry={h.e} bookName={active.length > 1 ? h.book : undefined} open={openId === h.e.id} ontoggle={() => (openId = openId === h.e.id ? null : h.e.id)} />
						{/each}
					</ul>
				{:else}
					<p class="hint">맞는 항목 없음</p>
				{/if}
			{:else}
				<p class="hint">걸린 책의 항목을 찾아 바로 고침. 새 항목은 로어북 화면에서</p>
			{/if}
		</section>
	{/if}
</div>

<style>
	.books {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.books li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 6px var(--space-2);
		border-radius: var(--radius-sm);
	}
	.books li.on {
		background: var(--surface);
	}
	.t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
		line-height: 1.35;
	}
	.t b {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	li:not(.on) .t b {
		color: var(--text-muted);
		font-weight: 500;
	}
	.t small {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.entries {
		margin: 0;
		padding: 0;
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
		text-decoration: underline;
	}
</style>
