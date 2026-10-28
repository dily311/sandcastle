<script lang="ts">
	import WidgetSetting from "./WidgetSettingSection.svelte";
	import ThemeSettingPage from "./ThemeSettingSection.svelte";
	import AppSettingSection from "./AppSettingSection.svelte";
	import Icon from "$/components/ui/Icon.svelte";

	// 탭 활성화 상태
	type TabsType = "app" | "theme" | "widget" | "data";
	interface Tab {
		type: TabsType;
		label: string;
		icon?: string;
	}
	const tabs: Tab[] = [
		{
			type: "app",
			label: "앱 설정",
		},
		{
			type: "theme",
			label: "테마 및 외관",
		},
		{
			type: "widget",
			label: "위젯",
		},
		{
			type: "data",
			label: "데이터 관리",
		},
	];
	let activeTab: TabsType = "app";
</script>

<div class="page settings-layout">
	<!-- Navigation (Desktop: Vertical Sidebar / Mobile: Horizontal Header) -->
	<nav class="settings-nav">
		<h2 class="nav-title">설정</h2>
		<div class="tab-group" role="group">
			{#each tabs as tab}
				<button class="tab-btn" class:active={activeTab === tab.type} onclick={() => (activeTab = tab.type)}>
					<span class="tab-icon">
						{#if tab.icon}
							<Icon name={tab.icon} />
						{/if}
					</span>
					<span class="tab-label">{tab.label}</span>
				</button>
			{/each}
		</div>
	</nav>

	<!-- Main Settings Panel -->
	<main class="settings-content">
		{#if activeTab === "app"}
			<AppSettingSection />
		{:else if activeTab === "theme"}
			<ThemeSettingPage />
		{:else if activeTab === "widget"}
			<WidgetSetting />
		{/if}
	</main>
</div>

<style>
	.settings-layout {
		display: flex;
		flex-direction: column;

		gap: 1.5rem;
	}

	.nav-title {
		font-size: 1.25rem;
		font-weight: 700;
		margin: 0 0 1rem 0;
		display: none;
	}

	.settings-nav {
		border-bottom: 2px solid var(--border-color, #e5e7eb);
	}

	.tab-group {
		display: flex;
		gap: 0.5rem;
	}

	.tab-btn {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		margin-bottom: -2px;
		font-size: 0.95rem;
		font-weight: 500;
		color: var(--text-muted, #6b7280);
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.tab-btn:hover {
		color: var(--text-color, #111827);
	}

	.tab-btn.active {
		color: var(--accent-color, #2563eb);
		border-bottom-color: var(--accent-color, #2563eb);
		font-weight: 600;
	}

	.settings-content {
		flex: 1;
	}

	/* Desktop View (Vertical Tabs) */
	@media (min-width: 1000px) {
		.settings-layout {
			flex-direction: row;
			align-items: flex-start;
			gap: 2rem;
		}

		.nav-title {
			display: block;
		}

		.settings-nav {
			width: 220px;
			flex-shrink: 0;
			border-bottom: none;
			border-right: 1px solid var(--border-color, #e5e7eb);
			padding-right: 1rem;
		}

		.tab-group {
			flex-direction: column;
			gap: 0.25rem;
		}

		.tab-btn {
			width: 100%;
			justify-content: flex-start;
			border-bottom: none;
			border-radius: 8px;
			margin-bottom: 0;
			padding: 0.65rem 0.85rem;
		}

		.tab-btn.active {
			background-color: var(--nav-bg-active);
			color: var(--text-accent);
			border-bottom-color: transparent;
		}
	}
</style>
