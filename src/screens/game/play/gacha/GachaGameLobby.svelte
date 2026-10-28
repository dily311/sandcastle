<script lang="ts">
  import { push } from 'svelte-spa-router';
	let { sessionId }: { sessionId: string } = $props();


  // 가챠 게임용 렌더 상태 (재화 및 메인 화면 캐릭터 정보)
  let playerState = $state({
    name: '아르카나 탐구자',
    level: 45,
    hardCurrency: 1850, // 보석/다이아
    softCurrency: 245000, // 골드
    stamina: 120,
    maxStamina: 150,
    currentSecretary: {
      name: '소네니 (Sonetto)',
      title: '재단 수석 정비사',
      portraitBg: 'bg-retro-art'
    }
  });

  let isGachaModalOpen = $state(false);
  let gachaResults = $state<string[]>([]);
  let isSummoning = $state(false);

  // 10연차 소환 시뮬레이션
  function handlePerformSummon(count: number) {
    isSummoning = true;
    setTimeout(() => {
      const pool = ['★5 콘블룸', '★4 레일라니', '★6 드루비스 III', '★3 백현실', '★4 에릭'];
      gachaResults = Array.from({ length: count }, () => pool[Math.floor(Math.random() * pool.length)]);
      isSummoning = false;
    }, 1500);
  }

  function handleGoStage() {
    push('/stage-select');
  }
</script>

<div class="gacha-lobby-layout">
  <!-- 1. 상단 재화 및 프로필 바 -->
  <header class="top-resource-bar">
    <div class="user-profile">
      <div class="avatar-frame">👤</div>
      <div class="user-info">
        <span class="name">{playerState.name}</span>
        <span class="lvl">Lv.{playerState.level}</span>
      </div>
    </div>

    <div class="resource-group">
      <div class="res-item">
        <span class="icon">⚡</span>
        <span class="val">{playerState.stamina} / {playerState.maxStamina}</span>
      </div>
      <div class="res-item">
        <span class="icon">💎</span>
        <span class="val">{playerState.hardCurrency}</span>
      </div>
      <div class="res-item">
        <span class="icon">🪙</span>
        <span class="val">{playerState.softCurrency}</span>
      </div>
    </div>
  </header>

  <!-- 2. 중앙 메인 캐릭터(비서) 및 배경 영역 -->
  <main class="center-stage">
    <div class="secretary-container">
      <!-- Pixi.js 또는 고해상도 일러스트가 들어갈 자리 -->
      <div class="character-illustration-placeholder">
        <div class="char-silhouette">✨ {playerState.currentSecretary.name}</div>
        <span class="char-title">{playerState.currentSecretary.title}</span>
      </div>
    </div>
  </main>

  <!-- 3. 우측/하단 네비게이션 메인 메뉴 -->
  <nav class="main-menu-dock">
    <button class="dock-btn story" onclick={handleGoStage}>
      <span class="icon">📖</span>
      <span class="label">스토리 / 스토리텔링</span>
    </button>
    <button class="dock-btn roster">
      <span class="icon">👥</span>
      <span class="label">캐릭터 / 육성</span>
    </button>
    <button class="dock-btn gacha" onclick={() => isGachaModalOpen = true}>
      <span class="icon">🔮</span>
      <span class="label">소환 (Gacha)</span>
    </button>
  </nav>

  <!-- 4. 소환(가챠) 팝업 모달 -->
  {#if isGachaModalOpen}
    <div class="gacha-modal-overlay">
      <div class="gacha-modal-content">
        <div class="modal-header">
          <h2>절찬 소환 진행 중</h2>
          <button class="close-btn" onclick={() => { isGachaModalOpen = false; gachaResults = []; }}>✖</button>
        </div>

        {#if isSummoning}
          <div class="summon-animation">
            <div class="crystal-spinning">🔮 의식이 거행되는 중...</div>
          </div>
        {:else if gachaResults.length > 0}
          <div class="results-grid">
            {#each gachaResults as res}
              <div class="result-card">
                <span class="card-name">{res}</span>
              </div>
            {/each}
          </div>
          <div class="modal-actions">
            <button class="summon-btn" onclick={() => handlePerformSummon(10)}>10회 더 소환</button>
          </div>
        {:else}
          <div class="banner-preview">
            <div class="banner-title">특급 캐릭터 의식 확률 UP</div>
            <p>미지의 통찰을 지닌 귀중한 인물을 영입하세요.</p>
            <div class="banner-actions">
              <button class="summon-btn" onclick={() => handlePerformSummon(1)}>1회 소환 (보석 180)</button>
              <button class="summon-btn primary" onclick={() => handlePerformSummon(10)}>10회 소환 (보석 1800)</button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .gacha-lobby-layout {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0f0e17;
    color: #fffffe;
    font-family: 'Pretendard', sans-serif;
    overflow: hidden;
  }

  /* 상단 재화 바 스타일 */
  .top-resource-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.8rem 2rem;
    background: rgba(22, 21, 33, 0.9);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    z-index: 10;
  }
  .user-profile { display: flex; align-items: center; gap: 0.8rem; }
  .avatar-frame { width: 40px; height: 40px; background: #2e2b44; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 1.2rem; border: 1px solid #7f5af0; }
  .user-info { display: flex; flex-direction: column; }
  .user-info .name { font-weight: bold; font-size: 0.9rem; }
  .user-info .lvl { font-size: 0.75rem; color: #94a1b2; }

  .resource-group { display: flex; gap: 1.5rem; }
  .res-item { display: flex; align-items: center; gap: 0.4rem; background: #1a1827; padding: 0.3rem 0.8rem; border-radius: 20px; border: 1px solid rgba(255,255,255,0.08); font-size: 0.85rem; font-family: monospace; }

  /* 중앙 스테이지 */
  .center-stage { flex: 1; position: relative; display: flex; justify-content: center; align-items: center; }
  .character-illustration-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 320px;
    height: 480px;
    background: linear-gradient(135deg, #242238, #161521);
    border-radius: 16px;
    border: 1px solid #7f5af0;
    box-shadow: 0 10px 30px rgba(127, 90, 240, 0.2);
  }
  .char-silhouette { font-size: 1.2rem; font-weight: bold; color: #fffffe; }
  .char-title { font-size: 0.8rem; color: #94a1b2; margin-top: 0.4rem; }

  /* 하단 도크 메뉴 */
  .main-menu-dock {
    position: absolute;
    bottom: 2rem;
    right: 2rem;
    display: flex;
    gap: 1rem;
    z-index: 10;
  }
  .dock-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 110px;
    height: 90px;
    background: #1a1827;
    border: 1px solid rgba(127, 90, 240, 0.4);
    border-radius: 12px;
    color: #fffffe;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }
  .dock-btn:hover { background: #7f5af0; border-color: #fffffe; transform: translateY(-4px); }
  .dock-btn .icon { font-size: 1.5rem; margin-bottom: 0.3rem; }
  .dock-btn .label { font-size: 0.75rem; font-weight: bold; }

  /* 가챠 모달 */
  .gacha-modal-overlay {
    position: absolute;
    inset: 0;
    background: rgba(15, 14, 23, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 50;
  }
  .gacha-modal-content {
    width: 650px;
    background: #1a1827;
    border: 1px solid #7f5af0;
    border-radius: 16px;
    padding: 2rem;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 40px rgba(0,0,0,0.8);
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 1rem; margin-bottom: 1.5rem; }
  .modal-header h2 { margin: 0; font-size: 1.2rem; color: #7f5af0; }
  .close-btn { background: transparent; border: none; color: #94a1b2; font-size: 1.2rem; cursor: pointer; }
  .close-btn:hover { color: #fffffe; }

  .banner-preview { text-align: center; padding: 2rem 0; }
  .banner-title { font-size: 1.3rem; font-weight: bold; margin-bottom: 0.5rem; }
  .banner-actions { display: flex; justify-content: center; gap: 1rem; margin-top: 2rem; }
  
  .summon-btn {
    background: #242238;
    color: #fffffe;
    border: 1px solid #7f5af0;
    padding: 0.8rem 1.5rem;
    border-radius: 8px;
    font-weight: bold;
    cursor: pointer;
    transition: background 0.2s;
  }
  .summon-btn.primary { background: #7f5af0; }
  .summon-btn:hover { opacity: 0.9; transform: translateY(-2px); }

  .summon-animation { text-align: center; padding: 4rem 0; font-size: 1.2rem; color: #7f5af0; animation: pulse 1s infinite alternate; }
  
  .results-grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 1rem; margin-bottom: 2rem; }
  .result-card { background: #242238; border: 1px solid #7f5af0; height: 120px; border-radius: 8px; display: flex; align-items: center; justify-content: center; padding: 0.5rem; text-align: center; font-size: 0.85rem; font-weight: bold; }

  @keyframes pulse {
    from { opacity: 0.6; transform: scale(0.98); }
    to { opacity: 1; transform: scale(1.02); }
  }
</style>