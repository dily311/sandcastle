<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { push } from 'svelte-spa-router';
  import * as PIXI from 'pixi.js';

  let { sessionId }: { sessionId: string } = $props();

  // 비주얼 노벨용 렌더 상태
  interface RenderState {
    locationId: string | null;
    mapId: string;
    zoneId: string | null;
    gameTime: number;
    weatherID: string;
    timePeriodId: string;
    currentSceneId: string | null;
    activeNpcId: string | null;
    flags: {
      currentSpeaker?: string;
      dialogueText?: string;
      choices?: { label: string; action: string }[];
      background?: string;
      characterSprite?: string;
      [key: string]: any;
    };
  }

  let renderState = $state<RenderState>({
    locationId: 'room_1',
    mapId: 'academy',
    zoneId: 'south_wing',
    gameTime: 540,
    weatherID: '맑음',
    timePeriodId: '오전',
    currentSceneId: 'scene_01',
    activeNpcId: 'chloe',
    flags: {
      currentSpeaker: '클로에',
      dialogueText: '“저기... 오늘 방과 후에 도서관에서 같이 공부하지 않을래? 새로 들어온 마법 서적을 함께 보고 싶어.”',
      choices: [
        { label: '“좋아, 함께 가자.”', action: 'choose_yes' },
        { label: '“미안하지만 오늘은 약속이 있어.”', action: 'choose_no' }
      ],
      background: 'bg_library',
      characterSprite: 'chloe_smile'
    }
  });

  let isHistoryOpen = $state(false);
  let dialogueHistory = $state<string[]>([
    '클로에: “안녕? 오늘도 날씨가 참 좋다그그그.”',
    renderState.flags.dialogueText || ''
  ]);
  let isLoading = $state(false);

  // --- Pixi.js 캔버스 영역 제어 ---
  let canvasContainer: HTMLDivElement;
  let pixiApp: PIXI.Application | null = null;
  let bgSprite: PIXI.Graphics | null = null;
  let charGraphics: PIXI.Graphics | null = null;

  onMount(async () => {
    pixiApp = new PIXI.Application();
    await pixiApp.init({
      background: '#09090b',
      resizeTo: canvasContainer,
      antialias: true,
    });

    canvasContainer.appendChild(pixiApp.canvas);

    // 1. 배경 레이어 (그라데이션/색상 시뮬레이션)
    bgSprite = new PIXI.Graphics();
    drawBackground();
    pixiApp.stage.addChild(bgSprite);

    // 2. 캐릭터 스프라이트 레이어 (중앙 하단 배치)
    charGraphics = new PIXI.Graphics();
    drawCharacter();
    pixiApp.stage.addChild(charGraphics);

    // 창 크기 변경 대응
    window.addEventListener('resize', handleResize);
  });

  onDestroy(() => {
    window.removeEventListener('resize', handleResize);
    if (pixiApp) {
      pixiApp.destroy(true, { children: true });
    }
  });

  function drawBackground() {
    if (!pixiApp || !bgSprite) return;
    bgSprite.clear();
    // 비주얼 노벨 배경 분위기 (어두운 판타지/아카데미 톤)
    bgSprite.rect(0, 0, pixiApp.screen.width, pixiApp.screen.height);
    bgSprite.fill({ color: 0x1e1b4b }); // 네이비/퍼플 톤 배경
  }

  function drawCharacter() {
    if (!pixiApp || !charGraphics) return;
    charGraphics.clear();
    const cx = pixiApp.screen.width / 2;
    const cy = pixiApp.screen.height / 2 + 30;

    // 캐릭터 실루엣 대체 그래픽 (실제 게임에서는 PIXI.Assets.load로 텍스처 로드)
    charGraphics.roundRect(cx - 70, cy - 120, 140, 240, 20);
    charGraphics.fill({ color: 0xf43f5e }); // 캐릭터 실루엣 색상
    charGraphics.stroke({ width: 4, color: 0xfbcfe8 });
  }

  function handleResize() {
    drawBackground();
    drawCharacter();
  }

  // --- 플레이어 상호작용 (대화 진행 및 선택지 클릭) ---
  async function handleAdvanceDialogue(actionPayload?: string) {
    if (isLoading) return;
    
    const command = actionPayload || 'next';
    try {
      isLoading = true;
      // const res = await invoke<GameResponse>('dispatch_game_command', { command: { sessionId, type: 'action', payload: command } });
      // renderState = res.state;

      // 시뮬레이션 갱신
      if (renderState.flags.dialogueText) {
        dialogueHistory = [...dialogueHistory, `${renderState.flags.currentSpeaker}: ${renderState.flags.dialogueText}`];
      }
    } catch (err) {
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  function handleExit() {
    push('/sessions');
  }
</script>

<div class="vn-layout">
  <!-- 상단 오버레이 메뉴바 (Svelte) -->
  <header class="vn-top-bar">
    <div class="game-title">✨ 마법 학교의 비밀 연대기</div>
    <div class="menu-buttons">
      <button class="menu-btn" onclick={() => isHistoryOpen = !isHistoryOpen}>로그</button>
      <button class="menu-btn" onclick={handleExit}>나가기</button>
    </div>
  </header>

  <!-- 중앙 메인 뷰: Pixi.js 캔버스 영역 -->
  <div class="vn-canvas-area" bind:this={canvasContainer}>
    <!-- 캔버스 위를 덮는 Svelte 선택지 오버레이 (선택지가 있을 때만 노출) -->
    {#if renderState.flags.choices && renderState.flags.choices.length > 0}
      <div class="choices-overlay">
        {#each renderState.flags.choices as choice}
          <button class="choice-card" onclick={() => handleAdvanceDialogue(choice.action)}>
            {choice.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 하단 고정 대화창 영역 (Svelte) -->
  <footer class="vn-dialogue-box" onclick={() => !renderState.flags.choices?.length && handleAdvanceDialogue()}>
    <div class="speaker-tag">
      {renderState.flags.currentSpeaker || '???'}
    </div>
    <div class="dialogue-text-wrapper">
      <p class="dialogue-content">{renderState.flags.dialogueText || ''}</p>
      <span class="next-indicator">▼</span>
    </div>
  </footer>

  <!-- 대화 로그 모달 (로그 버튼 클릭 시) -->
  {#if isHistoryOpen}
    <div class="history-modal" role="dialog" aria-modal="true" aria-label="대화 로그">
      <div class="history-content">
        <div class="history-header">
          <h3>대화 기록</h3>
          <button class="close-btn" onclick={() => isHistoryOpen = false}>✖</button>
        </div>
        <div class="history-list">
          {#each dialogueHistory as history}
            <div class="history-item">{history}</div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .vn-layout {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #000;
    color: #fff;
    font-family: 'Pretendard', sans-serif;
    overflow: hidden;
    user-select: none;
  }

  /* 상단 메뉴바 */
  .vn-top-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 50px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 1.5rem;
    background: linear-gradient(to bottom, rgba(0,0,0,0.8), rgba(0,0,0,0));
    z-index: 20;
  }
  .game-title { font-size: 0.95rem; font-weight: bold; color: #e2e8f0; text-shadow: 0 1px 3px rgba(0,0,0,0.8); }
  .menu-buttons { display: flex; gap: 0.5rem; }
  .menu-btn {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #cbd5e1;
    padding: 0.3rem 0.8rem;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    backdrop-filter: blur(4px);
  }
  .menu-btn:hover { background: rgba(30, 41, 59, 0.8); color: #fff; }

  /* 중앙 캔버스 영역 */
  .vn-canvas-area {
    flex: 1;
    width: 100%;
    position: relative;
    overflow: hidden;
  }

  /* 선택지 오버레이 */
  .choices-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    z-index: 10;
    animation: fadeIn 0.3s ease-out;
  }
  .choice-card {
    background: rgba(15, 23, 42, 0.85);
    border: 1px solid #38bdf8;
    color: #f8fafc;
    padding: 0.9rem 2.5rem;
    border-radius: 8px;
    font-size: 1.05rem;
    font-weight: bold;
    cursor: pointer;
    width: 380px;
    text-align: center;
    transition: all 0.2s;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }
  .choice-card:hover {
    background: #0284c7;
    border-color: #7dd3fc;
    transform: translateY(-2px);
  }

  /* 하단 대화창 영역 */
  .vn-dialogue-box {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 180px;
    background: rgba(15, 23, 42, 0.85);
    border-top: 2px solid rgba(56, 189, 248, 0.4);
    backdrop-filter: blur(10px);
    padding: 1.5rem 2.5rem;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    z-index: 20;
    cursor: pointer;
  }

  .speaker-tag {
    position: absolute;
    top: -22px;
    left: 2.5rem;
    background: #0284c7;
    color: white;
    padding: 0.3rem 1.2rem;
    border-radius: 6px;
    font-weight: bold;
    font-size: 0.95rem;
    box-shadow: 0 2px 6px rgba(0,0,0,0.3);
  }

  .dialogue-text-wrapper {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .dialogue-content {
    margin: 0;
    font-size: 1.15rem;
    line-height: 1.6;
    color: #f1f5f9;
  }
  .next-indicator {
    position: absolute;
    bottom: 0;
    right: 0;
    color: #38bdf8;
    font-size: 1rem;
    animation: bounce 1.5s infinite;
  }

  /* 대화 로그 모달 */
  .history-modal {
    position: absolute;
    inset: 0;
    background: rgba(0,0,0,0.7);
    backdrop-filter: blur(5px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 50;
  }
  .history-content {
    width: 600px;
    height: 450px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #1e293b;
  }
  .history-header h3 { margin: 0; font-size: 1rem; color: #f8fafc; }
  .close-btn { background: transparent; border: none; color: #94a3b8; font-size: 1rem; cursor: pointer; }
  .close-btn:hover { color: #fff; }
  .history-list {
    flex: 1;
    padding: 1.2rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .history-item { font-size: 0.9rem; color: #cbd5e1; line-height: 1.4; border-bottom: 1px solid #1e293b; padding-bottom: 0.6rem; }

  @keyframes bounce {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(4px); }
  }
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>