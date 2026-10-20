<script lang="ts">
	import type { link } from "$/app/routes";
	import { formatRelativeTime } from "$lib/utils/date";
	import { conversations } from "$/lib/chat/chatStore.svelte";
	import { push } from "svelte-spa-router";
	import Icon from "$/components/ui/Icon.svelte";

	const quickLinks: link[] = [
		{
			title: "새 채팅",
			desc: "캐릭터와 대화를 시작합니다.",
			color: "purple",
			href: "/chat",
			icon: "MessageCircle",
		},
		{
			title: "캐릭터 탐색",
			desc: "새로운 캐릭터를 찾아보세요.",
			color: "blue",
			href: "/character",
			icon: "Users",
		},
		{
			title: "AI 연결",
			desc: "사용할 AI API를 설정합니다.",
			color: "yellow",
			href: "/api",
			icon: "Zap",
		},
		{
			title: "프리셋",
			desc: "대화 설정을 관리합니다.",
			color: "green",
			href: "/preset",
			icon: "FileText",
		},
	];
</script>

<div class="page">
	<div class="content">
		<section class="welcome">
			<div class="welcome-text">
				<span class="eyebrow">WELCOME BACK</span>
				<h2>다시 이야기를 시작해보세요.</h2>
				<p>캐릭터와 대화하거나 새로운 AI 게임을 만들어보세요.</p>
			</div>

			<button class="primary-button">
				<Icon name="plus" size={17} />
				새 채팅
			</button>
		</section>

		<section class="quick-grid">
			{#each quickLinks as link}
				<button class="quick-card" onclick={() => push(link.href)}>
					<div class="quick-card-title">
						{#if link.icon}
							<div class={"quick-icon " + link.color}>
								<Icon name={link.icon} />
							</div>
						{/if}
						<div>
							<strong>{link.title}</strong>
							<span>{link.desc}</span>
						</div>
					</div>

					<div class="arrow-icon">
						<Icon name="ArrowRight" />
					</div>
				</button>
			{/each}
		</section>

		{#if conversations.value.length !== 0}
			<section class="section">
				<div class="section-header">
					<h3>최근 채팅</h3>
					<button onclick={() => push("/chat")}>전체 보기</button>
				</div>

				<div class="chat-list">
					{#each conversations.value.slice(0, 10) as chat}
						<button class="chat-item" onclick={() => push("/chat/" + chat.id)}>
							<div class="avatar">{chat.title.charAt(0)}</div>

							<div class="chat-info">
								<div class="chat-title">
									<strong>{chat.title}</strong>
									<span>{formatRelativeTime(chat.createdAt)}</span>
								</div>

								<p>{chat.mode}</p>
							</div>

							<Icon name="ArrowRight" />
						</button>
					{/each}
				</div>
			</section>
		{/if}
	</div>
</div>

<style>
	.page {
		height: 100%;
		background: var(--background-secondary);
		overflow-y: auto;
	}

	.welcome {
		min-height: 150px;
		padding: 28px;

		border: 1px solid var(--card-border-color);
		border-radius: 16px;

		background: var(--card-bg);

		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.welcome-text h2 {
		margin: 0.4rem 0;
	}

	.eyebrow {
		color: #7d75ee;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 1.2px;
	}

	.welcome p {
		margin: 0;
		color: var(--text-italic);
		font-size: 12px;
	}

	.primary-button {
		height: 38px;
		padding: 0 15px;

		display: flex;
		align-items: center;
		gap: 6px;

		border: 0;
		border-radius: 9px;

		color: white;
		background: var(--accent-color);

		cursor: pointer;
	}

	.quick-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 12px;
		margin-top: 18px;
	}

	.quick-card {
		min-height: 82px;
		padding: 15px;

		display: flex;
		align-items: center;
		justify-content: space-between;

		border: 1px solid var(--card-border-color);
		border-radius: 14px;

		background: var(--card-bg);

		cursor: pointer;
	}

	.quick-card:hover {
		border-color: #c9c6f6;
		background: var(--card-hover-bg);
	}

	.quick-card-title {
		display: flex;
		align-items: center;
		gap: 13px;
	}

	.quick-icon {
		width: 38px;
		height: 38px;
		flex-shrink: 0;
		color: var(--icon-color);
		border-radius: 10px;

		display: flex;
		align-items: center;
		justify-content: center;
	}

	.purple {
		background: #efeeff;
		color: #776ff0;
	}

	.blue {
		background: #edf4ff;
		color: #648ddd;
	}

	.yellow {
		background: #fff7df;
		color: #c3942f;
	}

	.green {
		background: #eaf8f0;
		color: #57a97b;
	}

	.quick-card strong {
		display: block;
		font-size: 13px;
		color: var(--text-color);
		font-weight: 600;
	}

	.quick-card span {
		display: block;
		margin-top: 3px;
		color: var(--text-muted);
		font-size: 11px;
	}

	.section {
		margin-top: 28px;
	}

	.section-header {
		margin-bottom: 10px;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	h3 {
		margin: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--text-color);
	}

	.section-header button {
		border: 0;
		background: transparent;
		color: var(--accent-color);
		font-size: 11px;
		cursor: pointer;
	}

	.chat-list {
		border: 1px solid var(--card-border-color);
		border-radius: 14px;
		background: var(--card-bg);
		overflow: hidden;
	}

	.chat-item {
		width: 100%;
		min-height: 72px;
		padding: 12px 15px;

		display: flex;
		align-items: center;
		gap: 12px;

		border: 0;
		border-bottom: 1px solid var(--border-color);

		cursor: pointer;
	}

	.chat-item:last-child {
		border-bottom: 0;
	}

	.chat-item:hover {
		background: var(--card-hover-bg);
	}

	.avatar {
		width: 42px;
		height: 42px;
		flex-shrink: 0;

		display: flex;
		align-items: center;
		justify-content: center;

		border-radius: 11px;
		background: #6f7685;
		color: white;
		font-size: 14px;
		font-weight: 600;
	}

	.chat-info {
		min-width: 0;
		flex: 1;
	}

	.chat-title {
		display: flex;
		justify-content: space-between;
	}

	.chat-title strong {
		font-size: 12px;
		color: var(--text-color);
	}

	.chat-title span {
		color: var(--text-faint);
		font-size: 10px;
	}

	.chat-info p {
		margin: 4px 0 0;

		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;

		color: var(--text-muted);
		font-size: 11px;
	}
</style>
