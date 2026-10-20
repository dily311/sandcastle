<script lang="ts">
	import { formatDateTime } from "$lib/utils/date";
	import { getMD } from "$/lib/markdown";
	import { messages } from "$/lib/chat/chatStore.svelte";
	import type { Message } from "$type/conv";
	import Avatar from "$components/avatar.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	let { msg, index }: { msg: Message; index: number } = $props();

	let swipeLength = $derived(msg.meta?.swipes?.length ?? 1);
	let swipeId = $derived(msg.meta?.swipeId ?? 0);
	let displaySwipeId = $derived(swipeId + 1);

	function handleNextSwipe() {
		if (displaySwipeId >= swipeLength) {
			messages.addSwipe(msg.id, "테스트");
		} else {
			messages.selectSwipe(msg.id, swipeId + 1);
		}
	}
	const md = getMD();
</script>

<div class="message-row {msg.role}">
	<Avatar avatar={msg.meta?.sender?.avatar ?? msg.meta?.sender?.color ?? ""} />
	<div class="bubble-wrap">
		<div class="meg-info-wrap">
			<div class="meg-info-title">
				<span class="sender-name">{msg.name}</span>
				<span class="meg-id badge">#{messages.activeLoadingMessageCount + index}</span>
			</div>
			<div class="meg-info">
				<span class="time-stamp">{formatDateTime(msg.createdAt)}</span>
			</div>
		</div>
		<ButtonGroup>
			<Button variant="ghost" size="icon" aria-label={"메시지 편집"}>
				<Icon name="Pen" />
			</Button>
		</ButtonGroup>
		<div class="bubble">
			{@html md.render(msg.content)}
			<div class="swipe-btns-wrap">
				{#if msg.role === "assistant"}
					<ButtonGroup>
						<Button
							variant="ghost"
							size="icon"
							class="action-btn clickable-icon"
							onclick={() => messages.selectSwipe(msg.id, swipeId - 1)}
							disabled={swipeId === 0}
						>
							<Icon name="ChevronLeft" />
						</Button>
						<div>
							{displaySwipeId} / {swipeLength}
						</div>
						<Button
							variant="ghost"
							size="icon"
							class="action-btn clickable-icon"
							onclick={() => handleNextSwipe()}
						>
							<Icon name="ChevronRight" />
						</Button>
					</ButtonGroup>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.message-row {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		max-width: 80%;
	}

	.message-row.user {
		align-self: flex-end;
		flex-direction: row-reverse;
	}

	.bubble-wrap {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		overflow: hidden;
	}

	.meg-info-wrap {
		font-size: var(--font-size-sm);
	}

	.meg-info-title {
		display: flex;
		align-items: center;
		margin-bottom: 0.2rem;
	}

	.sender-name {
		font-weight: 500;
	}

	.meg-id {
		margin-left: 4px;
	}

	.bubble {
		background: var(--bubble-bg);
		border: 1.5px solid var(--bubble-border-color);
		padding: 12px 16px;
		border-radius: 18px;
		border-top-left-radius: 4px;
		font-size: 14px;
		line-height: 1.5;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.02);
		overflow-wrap: anywhere;
	}

	:global(.bubble p:last-child) {
		margin-bottom: 0;
	}
	:global(.bubble p strong, .bubble p b) {
		font-weight: bold;
	}

	.message-row.user .bubble {
		background: var(--user-bubble-bg);
		border-color: var(--user-bubble-border-color);
		border-top-left-radius: 18px;
		border-top-right-radius: 4px;
	}

	.time-stamp {
		font-size: 10px;
		color: var(--text-sub);
		align-self: flex-end;
	}

	.typing-bubble {
		color: var(--accent-color);
		font-size: 13px;
		display: flex;
		gap: 6px;
	}

	.swipe-btns-wrap {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 0.6rem;
		--icon-size: 1rem;
	}
</style>
