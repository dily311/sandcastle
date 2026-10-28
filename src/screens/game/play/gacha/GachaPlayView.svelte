<script lang="ts">
  import { push } from 'svelte-spa-router';

  // 가챠 아이템 결과 타입 정의
  interface GachaItemResult {
    id: string;
    name: string;
    rarity: 'R' | 'SR' | 'SSR' | 'UR';
    isFeatured: boolean;
  }

  let { poolId, sessionId }: { poolId: string; sessionId: string } = $props();

  // 상태 관리 ('idle' = 대기, 'summoning' = 연출 중, 'results' = 결과 공개)
  let viewState = $state<'idle' | 'summoning' | 'results'>('idle');
  let drawnResults = $state<GachaItemResult[]>([]);
  let currentCurrency = $state({ diamond: 2500, ticket: 5 });

  // 가챠 실행 (1회 또는 10회)
  async function handleExecutePull(count: number) {
    const cost = count * 180;
    if (currentCurrency.diamond < cost) {
      alert('보석(Diamond)이 부족합니다!');
      return;
    }

    currentCurrency.diamond -= cost;
    viewState = 'summoning';

    // 1.5초간 가챠 연출(마법진/크리스탈 회전) 시뮬레이션
    setTimeout(() => {
      drawnResults = simulatePullResults(count);
      viewState = 'results';
    }, 1600);
  }

  // 확률에 따른 무작위 결과 생성 시뮬레이션 (엔진 API 연동 대체용)
  function simulatePullResults(count: number): GachaItemResult[] {
    const mockPool: { name: string; rarity: 'R' | 'SR' | 'SSR' | 'UR'; weight: number; isFeatured: boolean }[] = [
      { name: '노련한 사제', rarity: 'R', weight: 60, isFeatured: false },
      { name: '견습 마법사', rarity: 'R', weight: 60, isFeatured: false },
      { name: '왕립 기사단원', rarity: 'SR', weight: 25, isFeatured: false },
      { name: '숲의 궁수', rarity: 'SR', weight: 25, isFeatured: false },
      { name: '대현자 엘리시아', rarity: 'SSR', weight: 8, isFeatured: true },
      { name: '공허의 마녀 비비안', rarity: 'UR', weight: 2, isFeatured: true },
    ];

    const results: GachaItemResult[] = [];
    for (let i = 0; i < count; i++) {
      const totalWeight = mockPool.reduce((sum, item) => sum + item.weight, 0);
      let randomVal = Math.random() * totalWeight;
      let selected = mockPool[0];

      for (const item of mockPool) {
        if (randomVal < item.weight) {
          selected = item;
          break;
        }
        randomVal -= item.weight;
      }

      results.push({
        id: crypto.randomUUID(),
        name: selected.name,
        rarity: selected.rarity,
        isFeatured: selected.isFeatured
      });
    }
    return results;
  }

  function handleBackToLobby() {
    push('/lobby');
  }
</script>

<div class="gacha-play-container">
  <!-- 상단 헤더 바 -->
  <header class="gacha-header">
    <button class="back-btn" onclick={handleBackToLobby}>← 로비로</button>
    <div class="currency-display">
      <span class="res">💎 {currentCurrency.diamond}</span>
      <span class="res">🎟️ {currentCurrency.ticket}</span>
    </div>
  </header>

  <!-- 상태별 화면 분기 -->
  {#if viewState === 'idle'}
    <!-- 1. 대기 및 배너 소개 화면 -->
    <main class="banner-stage">
      <div class="banner-card-box">
        <div class="banner-badge">✨ PICK-UP EVENT</div>
        <h1>공허와 빛의 교차점</h1>
        <p>신규 UR 등급 「공허의 마녀 비비안」 출현 확률 대폭 상승!</p>
        
        <div class="pull-actions">
          <button class="pull-btn" onclick={() => handleExecutePull(1)}>
            <span class="title">단차 소환</span>
            <span class="cost">💎 180</span>
          </button>
          <button class="pull-btn primary" onclick={() => handleExecutePull(10)}>
            <span class="title">10연속 소환</span>
            <span class="cost">💎 1,800 (확정 포함)</span>
          </button>
        </div>
      </div>
    </main>

  {:else if viewState === 'summoning'}
    <!-- 2. 소환 연출 화면 (마법진 회전 & 빛 폭발 애니메이션) -->
    <div class="summon-casting-overlay">
      <div class="magic-circle">
        <div class="ring outer"></div>
        <div class="ring inner"></div>
        <div class="core-glow">🔮</div>
      </div>
      <p class="casting-text">소환의 의식을 거행하는 중...</p>
    </div>

  {:else if viewState === 'results'}
    <!-- 3. 결과 카드 등장 화면 -->
    <main class="results-stage">
      <h2>소환 결과</h2>
      <div class="cards-grid">
        {#each drawnResults as item, index}
          <div class="result-card-item rarity-{item.rarity.toLowerCase()}" style="animation-delay: {index * 0.1}s">
            {#if item.isFeatured}
              <div class="featured-tag">UP</div>
            {/if}
            <div class="rarity-star">{item.rarity}</div>
            <div class="card-name">{item.name}</div>
          </div>
        {/each}
      </div>
      <div class="results-actions">
        <button class="action-btn retry" onclick={() => handleExecutePull(10)}>다시 소환</button>
        <button class="action-btn confirm" onclick={() => viewState = 'idle'}>확인</button>
      </div>
    </main>
  {/if}
</div>

<style>
  .gacha-play-container {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #09090b;
    color: #f8fafc;
    font-family: 'Pretendard', sans-serif;
    overflow: hidden;
  }

  .gacha-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    background: rgba(15, 23, 42, 0.8);
    border-bottom: 1px solid #1e293b;
    z-index: 10;
  }
  .back-btn { background: #334155; color: white; border: none; padding: 0.4rem 0.8rem; border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-weight: bold; }
  .back-btn:hover { background: #475569; }
  .currency-display { display: flex; gap: 1.5rem; font-family: monospace; font-size: 0.95rem; }

  /* 배너 대기 화면 스타일 */
  .banner-stage {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    background: radial-gradient(circle at center, #1e1b4b 0%, #09090b 80%);
  }
  .banner-card-box {
    text-align: center;
    background: rgba(15, 23, 42, 0.75);
    border: 1px solid #4f46e5;
    padding: 3rem 4rem;
    border-radius: 20px;
    box-shadow: 0 20px 50px rgba(79, 70, 229, 0.3);
    backdrop-filter: blur(10px);
    max-width: 500px;
  }
  .banner-badge { display: inline-block; background: #4f46e5; color: #fff; font-size: 0.75rem; font-weight: bold; padding: 0.3rem 0.8rem; border-radius: 20px; margin-bottom: 1rem; }
  .banner-card-box h1 { margin: 0 0 0.8rem 0; font-size: 1.8rem; font-weight: 800; }
  .banner-card-box p { color: #94a3b8; font-size: 0.9rem; margin-bottom: 2rem; line-height: 1.5; }

  .pull-actions { display: flex; justify-content: center; gap: 1rem; }
  .pull-btn {
    background: #1e293b;
    border: 1px solid #475569;
    color: white;
    padding: 0.8rem 1.5rem;
    border-radius: 10px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    transition: all 0.2s;
  }
  .pull-btn.primary { background: #4f46e5; border-color: #818cf8; }
  .pull-btn:hover { transform: translateY(-3px); }
  .pull-btn .title { font-weight: bold; font-size: 0.95rem; }
  .pull-btn .cost { font-size: 0.75rem; color: #cbd5e1; font-family: monospace; }

  /* 소환 연출(캐스팅) 오버레이 스타일 */
  .summon-casting-overlay {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    background: #020617;
  }
  .magic-circle {
    position: relative;
    width: 140px;
    height: 140px;
    display: flex;
    justify-content: center;
    align-items: center;
    animation: pulseGlow 1.2s infinite alternate;
  }
  .ring {
    position: absolute;
    inset: 0;
    border: 3px dashed #6366f1;
    border-radius: 50%;
    animation: spinRing 4s linear infinite;
  }
  .ring.inner { inset: 15px; border-style: solid; border-color: #ec4899; animation-direction: reverse; animation-duration: 3s; }
  .core-glow { font-size: 2.5rem; z-index: 2; animation: bounceCore 0.6s infinite alternate; }
  .casting-text { margin-top: 2rem; font-size: 1.1rem; color: #a5b4fc; font-weight: bold; letter-spacing: 1px; animation: fadeInOut 1s infinite alternate; }

  /* 결과 카드 등장 화면 스타일 */
  .results-stage {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    padding: 2rem;
    background: radial-gradient(circle at center, #172554 0%, #09090b 90%);
  }
  .results-stage h2 { margin-bottom: 1.5rem; font-size: 1.5rem; color: #38bdf8; font-weight: 800; }
  
  .cards-grid {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
    max-width: 900px;
    margin-bottom: 2rem;
  }

  .result-card-item {
    position: relative;
    width: 130px;
    height: 190px;
    background: #1e293b;
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 1rem;
    box-shadow: 0 10px 25px rgba(0,0,0,0.5);
    border: 2px solid #475569;
    animation: cardPopIn 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
    transform-origin: center;
  }
  
  /* 등급별 카드 디자인 테두리 */
  .result-card-item.rarity-r { border-color: #64748b; background: linear-gradient(135deg, #1e293b, #0f172a); }
  .result-card-item.rarity-sr { border-color: #0ea5e9; background: linear-gradient(135deg, #0c4a6e, #0f172a); box-shadow: 0 0 15px rgba(14, 165, 233, 0.3); }
  .result-card-item.rarity-ssr { border-color: #f59e0b; background: linear-gradient(135deg, #78350f, #0f172a); box-shadow: 0 0 20px rgba(245, 158, 11, 0.4); }
  .result-card-item.rarity-ur { border-color: #f43f5e; background: linear-gradient(135deg, #881337, #0f172a); box-shadow: 0 0 25px rgba(244, 63, 94, 0.6); }

  .featured-tag { position: absolute; top: 6px; right: 6px; background: #f43f5e; color: white; font-size: 0.65rem; font-weight: bold; padding: 0.15rem 0.4rem; border-radius: 4px; }
  .rarity-star { font-size: 0.85rem; font-weight: 800; color: #fbbf24; }
  .card-name { font-size: 0.9rem; font-weight: bold; text-align: center; color: #f1f5f9; margin-bottom: 1rem; }

  .results-actions { display: flex; gap: 1rem; }
  .action-btn { padding: 0.7rem 1.8rem; border-radius: 8px; font-weight: bold; cursor: pointer; border: none; font-size: 0.95rem; transition: transform 0.15s; }
  .action-btn:hover { transform: translateY(-2px); }
  .action-btn.retry { background: #334155; color: white; border: 1px solid #475569; }
  .action-btn.confirm { background: #0ea5e9; color: white; }

  /* 애니메이션 키프레임 */
  @keyframes spinRing {
    to { transform: rotate(360deg); }
  }
  @keyframes pulseGlow {
    from { transform: scale(0.95); opacity: 0.8; }
    to { transform: scale(1.05); opacity: 1; }
  }
  @keyframes bounceCore {
    from { transform: translateY(0); }
    to { transform: translateY(-8px); }
  }
  @keyframes fadeInOut {
    from { opacity: 0.5; }
    to { opacity: 1; }
  }
  @keyframes cardPopIn {
    0% { opacity: 0; transform: scale(0.5) translateY(30px); }
    100% { opacity: 1; transform: scale(1) translateY(0); }
  }
</style>