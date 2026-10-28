<script lang="ts">
	import { conversations } from "$/lib/chat/chatStore.svelte";
	import { Search, Plus, MoreVertical, MessageCircle } from "@lucide/svelte";
	import { push } from "svelte-spa-router";
	import Avatar from "$components/avatar.svelte";
	let { params }: { params?: { characterId?: string } } = $props();

	let data = $derived(conversations.value.filter((c) => c.characterId === params?.characterId));
	let search = $state("");
</script>

<div class="content">
	<div class="page-title">
		<div>
			<h2>채팅</h2>
			<p>지금까지 나눈 대화를 확인합니다.</p>
		</div>

		<button class="primary" onclick={() => push("/character")}>
			<Plus size={16} />
			새 채팅
		</button>
	</div>

	<div class="toolbar">
		<div class="search">
			<Search size={16} />
			<input bind:value={search} placeholder="채팅 검색..." />
		</div>
	</div>

	<section class="list">
		{#each data as conv (conv.id)}
			<button class="chat" onclick={() => push("/chat/" + conv.id)}>
				<Avatar avatar={conv.title.charAt(0)} />

				<div class="chat-main">
					<div class="chat-top">
						<strong>{conv.title}</strong>
						<span>{conv.createdAt}</span>
					</div>

					<p>{conv.mode}</p>

					<div class="chat-bottom">
						<span>
							{conv.pinned}
						</span>
						<span>
							<MessageCircle size={12} />
							{conv.messageCount}개의 메시지
						</span>
					</div>
				</div>

				<MoreVertical size={17} />
			</button>
		{/each}
	</section>
</div>

<style>
	.page-title {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	h2 {
		margin: 0;
		font-size: 19px;
		color: var(--text-color);
	}

	.page-title p {
		margin: 5px 0 0;
		color: var(--text-muted);
		font-size: 11px;
	}

	.primary {
		height: 36px;
		padding: 0 13px;

		display: flex;
		align-items: center;
		gap: 6px;

		border: 0;
		border-radius: 8px;

		background: var(--accent-color);
		color: white;

		font-size: 11px;
		cursor: pointer;
	}

	.toolbar {
		margin: 18px 0 10px;
	}

	.search {
		height: 38px;
		padding: 0 11px;

		display: flex;
		align-items: center;
		gap: 8px;

		border: 1px solid var(--border-color);
		border-radius: 9px;

		color: var(--text-muted);
	}

	.search input {
		flex: 1;
		font-size: 11px;
	}

	.list {
		border: 1px solid var(--card-border-color);
		background: var(--card-bg);
		border-radius: 14px;
		overflow: hidden;
	}

	.chat {
		width: 100%;
		min-height: 88px;
		padding: 14px;

		display: flex;
		align-items: center;
		gap: 12px;

		border: 0;
		border-bottom: 1px solid var(--border-color);

		text-align: left;
		cursor: pointer;
	}

	.chat:last-child {
		border-bottom: 0;
	}

	.chat:hover {
		background: var(--card-hover-bg);
	}

	.chat-main {
		flex: 1;
		min-width: 0;
	}

	.chat-top {
		display: flex;
		justify-content: space-between;
	}

	.chat-top strong {
		color: var(--text-color);
		font-size: 12px;
	}

	.chat-top span {
		color: #a0a4ad;
		font-size: 9px;
	}

	.chat-main p {
		margin: 5px 0;

		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;

		color: var(--text-muted);
		font-size: 11px;
	}

	.chat-bottom {
		display: flex;
		align-items: center;
		color: #aaaeb5;
		font-size: 9px;
	}

	.chat-bottom span {
		display: flex;
		align-items: center;
		gap: 4px;
	}
</style>
