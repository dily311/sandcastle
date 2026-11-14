<script lang="ts">
	import { ArrowLeft, MessageSquarePlus, Search } from "@lucide/svelte";
	import { Avatar, Button, Input, Select } from "$shared/ui";
	import ChatDetail from "./ChatDetail.svelte";
	import ConvCard from "./ConvCard.svelte";
	import { characters, relative, store } from "./mock.svelte";

	// 대화: 캐릭터 → 그 캐릭터의 대화 목록 → 대화 상세
	let charId: string | null = $state(null);
	let convId: string | null = $state(null);

	let query = $state("");
	let sort: "recent" | "name" | "count" = $state("recent");
	const list = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return store.convs
			.filter((c) => c.charId === charId && (!q || c.title.toLowerCase().includes(q)))
			.sort((a, b) => Number(b.favorite) - Number(a.favorite) || (sort === "recent" ? a.ago - b.ago : sort === "name" ? a.title.localeCompare(b.title) : b.count - a.count));
	});
	const character = $derived(characters.find((c) => c.id === charId));
	const stats = (id: string) => {
		const cs = store.convs.filter((c) => c.charId === id);
		return { n: cs.length, last: Math.min(...cs.map((c) => c.ago)) };
	};
</script>

{#if charId && convId}
	<ChatDetail {convId} {charId} onback={() => (convId = null)} onopen={(id) => (convId = id)} />
{:else if charId && character}
	<div class="page">
		<header class="head">
			<Button variant="ghost" size="sm" onclick={() => (charId = null)}>
				{#snippet icon()}<ArrowLeft />{/snippet}
				캐릭터
			</Button>
			<div class="who">
				<Avatar name={character.name} size={56} />
				<div><h1>{character.name}</h1><p class="muted">{character.intro} · 대화 {list.length}개</p></div>
			</div>
			<div class="tools">
				<Input bind:value={query} placeholder="대화 검색" aria-label="대화 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<Select aria-label="정렬" bind:value={sort} options={[{ value: "recent", label: "최근 순" }, { value: "name", label: "이름 순" }, { value: "count", label: "메시지 많은 순" }]} />
				<Button variant="primary">
					{#snippet icon()}<MessageSquarePlus />{/snippet}
					새 대화
				</Button>
			</div>
		</header>
		<div class="cards">
			{#each list as c (c.id)}
				<ConvCard conv={c} onopen={() => (convId = c.id)} />
			{:else}
				<p class="none">{query ? "검색 결과 없음" : "아직 대화 없음"}</p>
			{/each}
		</div>
	</div>
{:else}
	<div class="page">
		<header class="head simple">
			<h1>대화</h1>
			<p class="muted">누구와 이야기할지 고르기</p>
		</header>
		<div class="chars">
			{#each characters as ch (ch.id)}
				{@const s = stats(ch.id)}
				<button class="char" onclick={() => (charId = ch.id)}>
					<Avatar name={ch.name} size={64} />
					<b>{ch.name}</b>
					<span class="muted">{ch.intro}</span>
					<span class="faint">대화 {s.n}개{#if s.n} · {relative(s.last)}{/if}</span>
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.page {
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 1100px;
		margin: 0 auto var(--space-5);
	}
	.head.simple {
		gap: var(--space-1);
	}
	.head :global(.btn) {
		align-self: flex-start;
	}
	.who {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}
	.tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 160px auto;
		gap: var(--space-2);
	}
	.cards,
	.chars {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: var(--space-3);
		max-width: 1100px;
		margin: 0 auto;
	}
	.chars {
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
	}
	.char {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-5) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		cursor: pointer;
		text-align: center;
		transition:
			border-color var(--duration-fast) var(--ease),
			transform var(--duration-fast) var(--ease);
	}
	.char:hover {
		border-color: var(--accent);
		transform: translateY(-1px);
	}
	.char b {
		margin-top: var(--space-2);
	}
	.char span {
		font-size: var(--text-sm);
	}
	.none {
		grid-column: 1 / -1;
		padding: var(--space-10);
		text-align: center;
		color: var(--text-faint);
	}
</style>
