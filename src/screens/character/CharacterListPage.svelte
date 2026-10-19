<script lang="ts">
	import { newConversation } from "$/lib/chat/chatStore.svelte";
	import { Plus, Search, MoreVertical, MessageCircle, Grid, List } from "@lucide/svelte";
	import { push } from "svelte-spa-router";
	import Avatar from "../chat/avatar.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";

	type ViewType = "grid" | "list";
	type Sort = "recent" | "name" | "favorite";

	let query = $state("");

	let view = $state<ViewType>("grid");
	let sort = $state<Sort>("recent");

	const setView = (changeView: ViewType) => (view = changeView);
</script>

<div class="content">
	<div class="page-title">
		<div>
			<h2>캐릭터</h2>
			<p>대화할 캐릭터를 관리하고 새로운 캐릭터를 만들어보세요.</p>
			<p>
				{characters.value.length}개의 캐릭터
			</p>
		</div>

		<button class="primary-btn" onclick={() => push("/character/edit")}>
			<Plus />
			캐릭터 만들기
		</button>
	</div>

	<div class="toolbar">
		<div class="segmented">
			<button class="clickable-icon" class:on={view === "grid"} onclick={() => setView("grid")} title="카드뷰">
				<Grid />
			</button>
			<button class="clickable-icon" class:on={view === "list"} onclick={() => setView("list")} title="리스트">
				<List />
			</button>
		</div>

		<div class="search">
			<Search />
			<input bind:value={query} placeholder="캐릭터 검색..." />
		</div>
	</div>

	{#if characters}
		{#if view === "grid"}
			<div class="grid">
				{#each characters.value as char}
					<article class="char-card">
						<Avatar avatar={char.avatar ?? char.color} />

						<div class="character-body">
							<div class="character-title">
								<h3>{char.name}</h3>

								<button class="more">
									<MoreVertical size={16} />
								</button>
							</div>

							<p class="character-des">{char.description}</p>

							<div class="character-footer">
								<span>
									<MessageCircle size={12} />
									{char.messageCount}개 채팅
								</span>

								<button
									class="chat-button"
									onclick={async () => {
										const convID = await newConversation(char.id);
										push("/chat/" + convID);
									}}
								>
									대화하기
								</button>
							</div>
						</div>
					</article>
				{/each}

				<button class="create-card">
					<div class="create-icon">
						<Plus size={21} />
					</div>

					<strong>새 캐릭터 만들기</strong>
					<span>나만의 AI 캐릭터를 만들어보세요.</span>
				</button>
			</div>
		{:else if view === "list"}
			<div class="list"></div>
		{/if}
	{/if}
</div>

<style>
	.content {
		max-width: none;
	}

	.page-title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		overflow: hidden;
	}

	.page-title p {
		margin: 5px 0 0;
		color: var(--text-muted);
		font-size: 11px;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.toolbar {
		margin: 18px 0 12px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.8rem;
	}

	.search {
		width: 100%;
		height: 38px;

		display: flex;
		align-items: center;
		gap: 8px;

		border: 1px solid var(--card-border-color);
		border-radius: 9px;

		background: var(--card-bg);
		color: #999ea7;
	}

	.search input {
		flex: 1;
		font-size: 11px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
		gap: 12px;
	}

	.char-card {
		min-height: 150px;
		padding: 15px;

		display: flex;
		gap: 13px;

		border: 1px solid var(--card-border-color);
		border-radius: 14px;

		background: var(--card-bg);
		transition:
			box-shadow 140ms ease,
			border-color 140ms ease,
			transform 140ms ease;
	}
	.char-card:hover {
		box-shadow: var(--shadow);
		border-color: transparent;
		transform: translateY(-1px);
	}

	.character-body {
		flex: 1;
		min-width: 0;
	}

	.character-title {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.character-des {
		display: -webkit-box;
		word-break: break-word;
		line-height: 1.5;
		text-overflow: ellipsis;
		overflow: hidden;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
	}

	.more {
		width: 26px;
		height: 26px;

		display: flex;
		align-items: center;
		justify-content: center;

		border: 0;
		border-radius: 6px;

		background: transparent;
		color: var(--text-muted);

		cursor: pointer;
	}

	.more:hover {
		background: #f0f1f4;
	}

	.character-body > p {
		margin: 7px 0 13px;
		color: var(--text-muted);
		font-size: 10px;
		line-height: 1.5;
	}

	.character-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.character-footer > span {
		display: flex;
		align-items: center;
		gap: 4px;

		color: var(--text-faint);
		font-size: 9px;
	}

	.chat-button {
		height: 28px;
		padding: 0 10px;

		border: 1px solid #dedcf8;
		border-radius: 7px;

		background: #f5f4ff;
		color: #746cf0;

		font-size: 10px;
		cursor: pointer;
	}

	.create-card {
		min-height: 150px;

		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;

		border: 1px dashed var(--card-border-color);
		border-radius: 14px;

		box-sizing: border-box;

		cursor: pointer;
	}

	.create-card:hover {
		background: var(--card-hover-bg);
		border-color: #aaa5f3;
	}

	.create-icon {
		width: 40px;
		height: 40px;

		display: flex;
		align-items: center;
		justify-content: center;

		border-radius: 11px;

		background: #efeeff;
		color: #766ef0;
	}

	.create-card strong {
		margin-top: 9px;
		color: #5b5f68;
		font-size: 11px;
	}

	.create-card span {
		margin-top: 4px;
		color: #9ca0a8;
		font-size: 9px;
	}

	@media (max-width: 720px) {
		.grid {
			grid-template-columns: 1fr;
		}
	}
</style>
