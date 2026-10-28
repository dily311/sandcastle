<script>
	// prettier-ignore
	import { BookOpen, ChevronRight, Compass, Heart, ImageIcon, MoreHorizontal, Plus, Sliders, Sparkles, User, UserCircle } from "@lucide/svelte";
	import { sidebarSidler } from "$lib/action/useSidebar";
	import { slide } from "svelte/transition";
	import Avatar from "$components/avatar.svelte";
	import { personas } from "$/lib/chat/personaStore.svelte";
	import { conversations } from "$/lib/chat/chatStore.svelte";

	let {
		isRightSidebarOpen: collapsed = $bindable(),
		// character,
		status = {
			hp: 82,
			maxHp: 100,
			mp: 34,
			maxMp: 50,
			level: 12,
			exp: 320,
			maxExp: 600,
		},
		inventory = [
			{ id: 1, name: "Mana Potion", count: 12, icon: "🧪" },
			{ id: 2, name: "Health Potion", count: 7, icon: "🍷" },
			{ id: 3, name: "Herbs", count: 3, icon: "🌿" },
			{ id: 4, name: "Meat", count: 1, icon: "🍖" },
		],
		quest = {
			title: "Find the ancient recipe",
		},
	} = $props();

	// AI 설정 상태값 예시
	let temperature = $state(0.7);
	let maxTokens = $state(2048);
	let tab = $state(1);
</script>

<!-- 오른쪽 사이드바 -->
{#if collapsed}
	<aside class="sidebar right-sidebar" class:collapsed transition:slide={{ axis: "x" }}>
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="resizer"
			role="separator"
			tabindex="0"
			{@attach sidebarSidler({
				axis: "right",
				isSidebarOpen: collapsed,
				toggleSidebar: () => (collapsed = !collapsed),
			})}
		></div>
		<div class="tab-header">
			<button class="clickable-icon add-btn" onclick={() => (tab = 1)}>
				<Plus />
			</button>
			<button class="clickable-icon add-btn" onclick={() => (tab = 2)}>
				<Plus />
			</button>
		</div>
		<div class="tab-container">
			<div data-tab="1" class="tab-content" class:active={tab === 1}>
				<!-- 현재 캐릭터 카드 -->
				<section class="card">
					<div class="card-title">
						<span class="dot">
							<User />
						</span>
						<span>현재 캐릭터</span>
					</div>
					{#if conversations.activeCharacter}
						<div class="character-profile-card-wrap">
							<div class="character-profile-card">
								<Avatar
									avatar={conversations.activeCharacter.avatar ?? conversations.activeCharacter.color}
								/>
								<div class="profile-info">
									<h4>{conversations.activeCharacter.name}</h4>
									<div class="profile-desc">
										<p>{conversations.activeCharacter.description}</p>
									</div>
								</div>
							</div>
							<button class="card-action-btn">
								<Plus />
							</button>
						</div>
					{/if}
				</section>

				<!-- 현재 페르소나 카드 -->
				<section class="card">
					<div class="card-title">
						<span class="icon">
							<UserCircle />
						</span>
						<span>현재 페르소나</span>
					</div>
					{#if personas.activePersona}
						<div class="persona-card">
							<Avatar avatar={personas.activePersona.avatar ?? personas.activePersona.color} />
							<div class="persona-info">
								<h4>{personas.activePersona.name}</h4>
								<p>{personas.activePersona.description}</p>
							</div>
						</div>
						<button class="card-action-btn">
							변경하기 <ChevronRight size={16} />
						</button>
					{/if}
				</section>

				<!-- 대화 파라미터 / AI 설정 -->
				<section class="card">
					<div class="card-title">
						<Sliders size={14} />
						<span>대화 설정</span>
					</div>
					<div class="setting-group">
						<div class="setting-item">
							<div class="setting-label">
								<span>게임 모드</span>
							</div>
							<select name="" id="">
								<option value="chat">채팅</option>
								<option value="text">텍스트게임</option>
								<option value="VN">비주얼노벨</option>
								<option value="RPG">RPG</option>
							</select>
						</div>
						<div class="setting-item">
							<div class="setting-label">
								<span>프리셋 변경</span>
							</div>
							<select name="" id="">
								<option value="프리셋1">프리셋1</option>
								<option value="프리셋2">프리셋2</option>
								<option value="프리셋3">프리셋3</option>
								<option value="프리셋4">프리셋4</option>
								<option value="프리셋7">프리셋7</option>
							</select>
						</div>
						<div class="setting-item">
							<div class="setting-label">
								<span>창의성 (Temperature)</span>
								<span class="value">{temperature}</span>
							</div>
							<input
								type="range"
								min="0"
								max="1"
								step="0.05"
								bind:value={temperature}
								class="pastel-slider"
							/>
						</div>

						<div class="setting-item">
							<div class="setting-label">
								<span>최대 응답 길이</span>
								<span class="value">{maxTokens}</span>
							</div>
							<input
								type="range"
								min="256"
								max="4096"
								step="128"
								bind:value={maxTokens}
								class="pastel-slider"
							/>
						</div>
					</div>
				</section>

				<!-- 갤러리 / 모아보기 -->
				<section class="card gallery-section">
					<div class="card-title">
						<ImageIcon />
						<span>생성된 일러스트</span>
					</div>
					<div class="image-grid">
						<div class="grid-item empty"><span>+</span></div>
						<div class="grid-item empty"><span>+</span></div>
						<div class="grid-item empty"><span>+</span></div>
					</div>
				</section>
			</div>
			<div data-tab="2" class="tab-content" class:active={tab === 2}>
				<!-- 2. STATUS 스탯 카드 -->
				{#if status}
					<section class="card status-card">
						<h4 class="card-title">STATUS</h4>

						<div class="stat-rows">
							<!-- HP -->
							<div class="stat-row">
								<span class="stat-label">HP</span>
								<div class="bar-bg">
									<div
										class="bar-fill hp-fill"
										style="width: {(status.hp / status.maxHp) * 100}%"
									></div>
								</div>
								<span class="stat-value">{status.hp} / {status.maxHp}</span>
							</div>

							<!-- MP -->
							<div class="stat-row">
								<span class="stat-label">MP</span>
								<div class="bar-bg">
									<div
										class="bar-fill mp-fill"
										style="width: {(status.mp / status.maxMp) * 100}%"
									></div>
								</div>
								<span class="stat-value">{status.mp} / {status.maxMp}</span>
							</div>

							<!-- Lv -->
							<div class="stat-row level-row">
								<span class="stat-label">Lv.</span>
								<div class="level-value">{status.level}</div>
							</div>

							<!-- Exp -->
							<div class="stat-row">
								<span class="stat-label">Exp</span>
								<div class="bar-bg">
									<div
										class="bar-fill exp-fill"
										style="width: {(status.exp / status.maxExp) * 100}%"
									></div>
								</div>
								<span class="stat-value">{status.exp} / {status.maxExp}</span>
							</div>
						</div>
					</section>
				{/if}
				<!-- 3. INVENTORY 인벤토리 카드 -->
				{#if inventory}
					<section class="card inventory-card">
						<h4 class="card-title">INVENTORY</h4>
						<div class="inventory-grid">
							{#each inventory as item}
								<div class="item-slot" title={item.name}>
									<span class="item-icon">{item.icon}</span>
									<span class="item-count">{item.count}</span>
								</div>
							{/each}
							<button class="item-slot more-btn" aria-label="More items">
								<MoreHorizontal />
							</button>
						</div>
					</section>
				{/if}
				<!-- 4. QUEST 퀘스트 카드 -->
				{#if quest}
					<section class="card quest-card">
						<h4 class="card-title">QUEST</h4>
						<p class="quest-sub-title">Current Objective</p>
						<div class="quest-box">
							<Compass />
							<span class="quest-text">{quest.title}</span>
							<Sparkles />
						</div>
					</section>
				{/if}

				<!-- 대화 기록 설명 -->
				<section class="card">
					<div class="card-title">
						<BookOpen />
						<span>시나리오 요약</span>
					</div>
					<div class="note-box">
						<p>요약 내용</p>
					</div>
				</section>
			</div>
		</div>
	</aside>
{/if}

<style>
	.right-sidebar {
		width: var(--sidebar-width);
		min-width: 100px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		position: relative;
	}

	/* 메모 박스 */
	.note-box {
		background: var(--color-20);
		border: 1px solid var(--border-color);
		border-radius: 12px;
		padding: 10px 12px;
		font-size: var(--font-size-sm);
		line-height: 1.5;
		color: var(--text-color);
	}

	.affection-label {
		font-size: var(--font-size-sm);
		color: var(--pink-accent);
		font-weight: 600;
		margin-bottom: 4px;
		display: flex;
		justify-content: space-between;
	}

	/* 사이드바 메인 레이아웃 */
	.tab-header {
		height: var(--header-height);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--border-color);
		background: var(--header-bg);
		padding: 0 7rem 0 0.95rem;
		gap: 0.8rem;
	}

	.tab-header .clickable-icon {
		app-region: no-drag;
	}

	.tab-container {
		overflow-y: auto;
	}
	.tab-content {
		display: none;
		flex-direction: column;
		flex: 1;
		gap: 12px;
		padding: 0.8rem;
	}
	.tab-content.active {
		display: flex;
	}
	/* 프로필 카드 내부 */
	.character-profile-card {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 12px;
		margin-bottom: 8px;
	}

	.avatar-frame {
		width: 4rem;
		height: 4rem;
		border-radius: 16px;
		overflow: hidden;
		background: #f7f7ff;
	}

	.avatar-frame img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.profile-info h4 {
		margin: 4px 0 2px 0;
		margin-bottom: 8px;
	}

	.profile-desc p {
		font-size: var(--font-size-sm);
		color: var(--text-muted);
	}
	.card-action-btn {
		width: 100%;
		padding: 8px;
		border-radius: 12px;
		border: 1px solid var(--border-color);
		background: var(--color-20);
		color: var(--text-color);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		transition: all 0.2s;
	}

	.card-action-btn:hover {
		background: var(--background-secondary);
		border-color: var(--accent-color);
	}
	/* 대화 설정 슬라이더 */
	.setting-group {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.setting-item {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.setting-label {
		display: flex;
		justify-content: space-between;
		font-size: var(--font-size-sm);
		font-weight: 600;
		color: var(--text-color);
	}

	.setting-label .value {
		color: var(--accent-color);
		font-weight: 700;
	}
	/* 갤러리 그리드 */
	.image-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 8px;
	}

	.grid-item {
		aspect-ratio: 1;
		border-radius: 10px;
		background: var(--button-bg);
		border: 1px dashed var(--border-color);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		font-size: 16px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.grid-item:hover {
		background: var(--background-secondary);
		border-color: var(--accent-color);
		color: var(--accent-color);
	}

	/* ================= 1. 프로필 카드 ================= */
	.persona-card {
		display: flex;
		gap: 12px;
		margin-bottom: 8px;
	}

	/* ================= 바(Bar) 공통 스타일 ================= */
	.bar-bg {
		width: 100%;
		height: 8px;
		background-color: #eef0f8;
		border-radius: 4px;
		overflow: hidden;
	}

	.bar-fill {
		height: 100%;
		border-radius: 4px;
		transition: width 0.3s ease;
	}

	.affection-fill {
		background: linear-gradient(90deg, #ff9ebb, #ff7597);
	}
	.hp-fill {
		background: linear-gradient(90deg, #ff8a9e, #ff5271);
	}
	.mp-fill {
		background: linear-gradient(90deg, #62b3ff, #3b82f6);
	}
	.exp-fill {
		background: linear-gradient(90deg, #a78bfa, #8b5cf6);
	}

	/* ================= 2. STATUS 스탯 카드 ================= */
	.stat-rows {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.stat-row {
		display: grid;
		grid-template-columns: 32px 1fr auto;
		align-items: center;
		gap: 8px;
		font-size: var(--font-size-sm);
		font-weight: 700;
	}

	.stat-label {
		color: var(--text-muted);
	}

	.stat-value {
		color: var(--text-italic);
		font-size: var(--font-size-sm);
		font-weight: 600;
		min-width: 50px;
		text-align: right;
	}

	/* 레벨 행 특별 처리 */
	.stat-row.level-row {
		display: flex;
		justify-content: space-between;
	}

	.level-value {
		color: #8b5cf6;
		font-weight: 800;
		font-size: 13px;
	}

	/* ================= 3. INVENTORY 카탈로그 ================= */
	.inventory-grid {
		display: flex;
		gap: 6px;
	}

	.item-slot {
		padding: 0.4rem;
		aspect-ratio: 1;
		background: var(--color-10);
		border: 1.5px solid var(--border-color);
		border-radius: 12px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.2rem;
		justify-content: center;
		position: relative;
		cursor: pointer;
		transition: all 0.2s;
	}

	.item-slot:hover {
		border-color: #8b85f9;
		background: var(--color-00);
		transform: translateY(-2px);
	}

	.item-icon {
		font-size: 18px;
	}

	.item-count {
		position: absolute;
		bottom: 2px;
		font-size: 10px;
		font-weight: 700;
		color: #64648c;
	}

	.more-btn {
		color: var(--text-italic);
	}

	/* ================= 4. QUEST 퀘스트 카드 ================= */
	.quest-sub-title {
		font-size: var(--font-size-sm);
		color: var(--text-muted);
		margin: -8px 0 8px 0;
	}

	.quest-box {
		background: var(--color-10);
		border-radius: var(--radius-md);
		padding: 10px 12px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.quest-text {
		font-size: var(--font-size-sm);
		flex: 1;
	}
</style>
