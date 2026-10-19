<script lang="ts">
	import { _ } from "svelte-i18n";
	import ChatStateSidebar from "$screens/chat/ChatStateSidebar.svelte";
	import MessageForm from "$screens/chat/MessageForm.svelte";
	import MessageIndicator from "$screens/chat/MessageIndicator.svelte";
	import { conversations, messages, openConversation } from "$/lib/chat/chatStore.svelte";
	import MessageBubble from "$screens/chat/MessageBubble.svelte";
	import { pop, router } from "svelte-spa-router";
	import { tick } from "svelte";
	import Icon from "$/components/Icon.svelte";

	let currentMode = $state(true); // true 시 대화 모드 / false 시 읽기 모드

	let params: string | null;

	$effect(() => {
		if (router.params && 'conversationId' in router.params) {
			params = router.params?.conversationId ?? null;
			if (params) {
				openConversation(params);
			}
		}
		return;
	});

	let isRightSidebarOpen = $state(false); // 오른쪽 사이드바 열림/닫힘 상태
	let bgImg = false;
	let chatBoxElement: HTMLElement | null = $state(null);
	let isLoadingMore = $state(false);
	let autoScroll = $state(true);

	function scrollToBottom() {
		setTimeout(() => {
			if (chatBoxElement) {
				chatBoxElement.scrollTop = chatBoxElement.scrollHeight;
			}
		}, 50);
	}

	async function handleScroll() {
		if (!chatBoxElement) return;
		// 저장된 메시지를 모두 로드 했을 경우 실행되지 않게 하기
		if (conversations.activeConv?.messageCount === messages.value.length) return;

		// 1. 상단에 도달하여 이전 메시지 로드
		if (chatBoxElement.scrollTop < 60 && !isLoadingMore && messages.value.length > 0) {
			isLoadingMore = true;
			const prevHeight = chatBoxElement.scrollHeight;
			const prevScrollTop = chatBoxElement.scrollTop;

			await messages.loadForConversation(conversations.activeConv!.id, {
				before: messages.value[0].createdAt,
			});
			await tick();

			// 이전 높이와 비교해 스크롤 위치 정교하게 보정
			chatBoxElement.scrollTop = chatBoxElement.scrollHeight - prevHeight + prevScrollTop;

			// 로드가 끝난 직후 바로 isLoadingMore를 풀지 않고 약간의 딜레이를 주거나
			// tick 이후에 안전하게 해제하여 effect 충돌 방지
			isLoadingMore = false;

			return;
		}

		// 2. 평소 스크롤 위치에 따른 autoScroll 여부 갱신
		autoScroll = chatBoxElement.scrollHeight - chatBoxElement.scrollTop - chatBoxElement.clientHeight < 100;
	}

	// 3. 메시지 변경 감지 개선 (messages.value 전체 대신 마지막 메시지 변화나 길이를 감지)
	let lastMessageId = $derived(messages.value[messages.value.length - 1]?.id);

	$effect(() => {
		// 마지막 메시지 참조 (의존성 트리거용)
		lastMessageId;

		// 로딩 중이거나 위로 스크롤해서 이전 데이터를 보는 중이면 절대 밑으로 내리지 않음
		if (isLoadingMore) return;

		// 새 메시지가 왔을 때 autoScroll이 true이거나, 활성 로딩 메시지가 있는 경우에만 하단 이동
		if (autoScroll || conversations.isChattingLoading) {
			scrollToBottom();
		}
	});
</script>

{#if conversations.activeConv}
	<div class="main-content-layout" class:isRightSidebarOpen>
		<main class="chat-container" class:cover-img={bgImg}>
			<!-- 상단 헤더 -->
			<header class="chat-header">
				<div class="header-left">
					<button type="button" aria-label={$_("chat.back")} class="back-btn clickable-icon" onclick={() => pop()}>
						<Icon name={"ChevronLeft"} />
					</button>
					<h3 class="chat-title-wrap">
						<span class="chat-title">{conversations.activeConv.title}</span>
					</h3>
				</div>
				<div class="header-right">
					<button
						type="button"
						aria-label={currentMode ? $_("chat.chatMode") : $_("chat.readMode")}
						class="pill-btn clickable-icon"
					>
						<Icon name={"BookOpenIcon"} />
					</button>
					<button type="button" aria-label={$_("chat.viewAction")} class="pill-btn clickable-icon">
						<Icon name={"EllipsisVertical"} />
					</button>
					<!-- 오른쪽 사이드바 토글 버튼 -->
					<button
						type="button"
						aria-label={$_("chat.rightSidebar")} 
						class="clickable-icon sidebar-toggle-btn"
						onclick={() => (isRightSidebarOpen = !isRightSidebarOpen)}
					>
						<Icon name={"PanelRight"} />
					</button>
				</div>
			</header>

			<div class="chat-content">
				<!-- 메시지 리스트 -->
				<div class="message-list" bind:this={chatBoxElement} onscroll={handleScroll}>
					{#each messages.value as msg, index (msg.id)}
						<MessageBubble {msg} {index} />
					{/each}

					{#if conversations.isChattingLoading}
						<MessageIndicator />
					{/if}
				</div>

				<!-- 하단 입력창 -->
				<div class="message-form-wrap">
					<MessageForm conversation={conversations.activeConv} character={conversations.activeCharacter} />
				</div>
			</div>
		</main>

		<ChatStateSidebar bind:isRightSidebarOpen />
	</div>
{:else}
	<div class="empty">선택된 대화방이 없습니다.</div>
{/if}

<style>
	.empty {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	/* 전체 컨테이너 (타이틀바/푸터 제외 영역) */
	.main-content-layout {
		display: flex;
		height: 100%;
	}

	/* ================= 2. 메인 채팅 영역 ================= */
	.chat-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.chat-container.cover-img {
		background: url("") no-repeat center;
		background-size: cover;
	}
	/* 헤더 */
	.chat-header {
		height: var(--header-height);
		flex: 0 0 var(--header-height);
		padding: 0 1rem;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0 7rem 0 0.95rem;
		background: var(--header-bg);
		white-space: nowrap;
	}
	.chat-header .clickable-icon {
		app-region: no-drag;
	}

	.isRightSidebarOpen .chat-header {
		padding: 0 0.95rem;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.back-btn {
		color: var(--text-color);
	}

	.chat-title {
		font-size: 0.9rem;
		font-weight: 700;
	}

	.chat-title-wrap .subtitle {
		font-size: 0.75rem;
		font-weight: 400;
		font-size: var(--font-size-sm);
		color: var(--text-muted);
	}

	.header-right {
		display: flex;
		align-items: center;
	}

	.chat-content {
		position: relative;
		max-width: 1024px;
		width: 100%;
		flex: 1;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	/* 메시지 영역 */
	.message-list {
		width: 100%;
		padding: 20px;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1.8rem;
		overflow: hidden auto;
	}

	.message-form-wrap {
		max-width: 1024px;
		width: 100%;
	}
</style>
