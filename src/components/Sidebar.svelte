<script lang="ts">
	import { PanelLeft, Pin, X } from "@lucide/svelte";
	import { navigation } from "$/app/routes";
	import { conversations, deleteConversation } from "$/lib/chat/chatStore.svelte";
	import { sidebarSidler } from "$lib/action/useSidebar";
	import { tooltipTrigger } from "$lib/stores/tooltip.svelte";
	import { push, router } from "svelte-spa-router";
	import { slide } from "svelte/transition";
	import Icon from "./ui/Icon.svelte";
	import Button from "./ui/button/Button.svelte";
	import ButtonGroup from "./ui/button/ButtonGroup.svelte";
	import { cn } from "$/lib/utils/cn";
	import IconButton from "./ui/button/IconButton.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";

	let collapsed = $state(true);
	const toggleLeft = () => (collapsed = !collapsed);
</script>

<aside class="sidebar left-sidebar" class:collapsed transition:slide={{ axis: "x" }}>
	{#if collapsed}
		<div class="sidebar-header">
			<IconButton size="lg" class="sidebar-toggle-btn" onclick={() => toggleLeft()}>
				<Icon name={"PanelLeft"} />
			</IconButton>
		</div>

		<div class="sidebar-contents">
			<div class="sidebar-nav">
				{#each navigation as page}
					<button
						class={cn("nav-item", router.location === page.link && "active")}
						onclick={() => push(page.link)}
						use:tooltipTrigger={{ label: page.label, delayDuration: 300 }}
					>
						{#if page.icon}
							<Icon name={page.icon} />
						{/if}
					</button>
				{/each}
			</div>
		</div>
	{:else}
		<div class="sidebar-header">
			<div></div>
			<IconButton size="lg" class="sidebar-toggle-btn" onclick={() => toggleLeft()}>
				<Icon name={"PanelLeft"} />
			</IconButton>
		</div>

		<div class="sidebar-contents">
			<div class="sidebar-nav">
				{#each navigation as page}
					<button
						class={cn("nav-item", router.location === page.link && "active")}
						onclick={() => push(page.link)}
					>
						{#if page.icon}
							<Icon name={page.icon} />
							{page.label}
						{/if}
					</button>
				{/each}
			</div>

			<div class="sidebar-section favorites">
				<div class="section-label">즐겨찾기</div>
				{#if characters.favorites.length === 0}
					<div class="section-empty">★ 여기에 표시됩니다.</div>
				{:else}
					<div class="section-content">
						<ul class="conv-list">
							{#each characters.favorites as char}
								<li class="conv-item">
									<button
										class="conv-title"
										class:active={router.location === `/character/${char.id}`}
										onclick={() => push(`/character/${char.id}`)}
									>
										{char.name}
									</button>
								</li>
							{/each}
						</ul>
					</div>
				{/if}
			</div>

			<div class="sidebar-section recent">
				<div class="section-label">최근 대화</div>
				{#if conversations.value.length === 0}
					<div class="section-empty">아직 대화가 없습니다.</div>
				{/if}
				<div class="section-content">
					<ul class="conv-list">
						{#each conversations.value as chat}
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
							<li class="conv-item">
								<button
									class="conv-title"
									class:active={router.location === `/chat/${chat.id}`}
									onclick={() => push(`/chat/${chat.id}`)}
								>
									<strong>{chat.title}</strong>
									<span>{chat.id}</span>
								</button>
								<ButtonGroup>
									<IconButton size="sm">
										<Pin />
									</IconButton>
									<IconButton size="sm" onclick={() => deleteConversation(chat.id)}>
										<X />
									</IconButton>
								</ButtonGroup>
							</li>
						{/each}
					</ul>
				</div>
			</div>
		</div>

		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="resizer left"
			role="separator"
			tabindex="0"
			{@attach sidebarSidler({
				axis: "left",
				isSidebarOpen: collapsed,
				toggleSidebar: () => (collapsed = !collapsed),
			})}
		></div>
	{/if}
</aside>

<style>
	.sidebar {
		position: relative;
		width: 240px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		background-color: var(--sidebar-bg);
		/* transition: width, 0.3s; */
		user-select: none;
		--icon-size: 1rem;
	}
	.sidebar.collapsed {
		flex: 0 0 var(--ribbon-width);
		border-right: 1px solid var(--border-color);
	}

	.sidebar-header {
		display: flex;
		justify-content: space-between;
	}

	.sidebar.collapsed .sidebar-header {
		justify-content: center;
		border-bottom: 1px solid var(--border-color);
	}

	:global(.sidebar .sidebar-toggle-btn) {
		padding: 8px;
		color: var(--icon-color);
		cursor: pointer;
		app-region: no-drag;
	}

	:global(body.is-frameless .sidebar .sidebar-toggle-btn) {
		height: var(--header-height);
	}

	.sidebar-contents {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		overflow: hidden;
	}

	.sidebar-nav {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 0.4rem;
		gap: 2px;
	}

	.nav-item {
		width: 100%;
		padding: 0.3rem 0.6rem;
		border-radius: 0.4rem;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		color: var(--nav-text);
		cursor: pointer;
	}

	.nav-item:hover {
		background: var(--nav-bg-hover);
	}
	.nav-item.active {
		font-weight: 500;
		color: var(--nav-text-active);
		background: var(--nav-bg-active);
	}

	.section-content {
		overflow-y: auto;
	}

	.conv-list {
		display: flex;
		flex-direction: column;
		align-items: center;
		font-size: 0.75rem;

		gap: 4px;
		padding: 0.3rem 0.6rem;
	}
	.conv-list .conv-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		gap: 4px;
		overflow: hidden;
	}

	.conv-title {
		color: var(--nav-text);
		padding: 0.3rem 0.6rem;
		border-radius: 0.4rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.conv-title:hover {
		font-weight: 500;
		color: var(--nav-text-active);
		background: var(--nav-bg-active);
	}

	.sidebar-section {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		gap: 4px;
	}
	.sidebar-section.recent {
		flex: 1;
	}

	.section-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.05rem;
		text-transform: uppercase;
		color: var(--text-italic);
		padding: 6px 8px 3px;
		padding: 0.4rem 1rem;
	}

	.section-empty {
		font-size: 11px;
		padding: 0.4rem 1.12rem;
		color: var(--text-italic);
	}

	.sidebar-footer {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-top: 1px solid var(--border-color);
		color: var(--text-muted);
	}
</style>
