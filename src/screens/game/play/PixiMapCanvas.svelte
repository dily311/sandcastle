<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as PIXI from 'pixi.js';

  type Props = {
    mapId: string;
    locationId: string | null;
    activeNpcId: string | null;
  };
  let { mapId, locationId, activeNpcId }: Props = $props();

  let canvasContainer: HTMLDivElement;
  let app: PIXI.Application | null = null;
  let characterSprite: PIXI.Graphics | null = null;

  onMount(async () => {
    // Pixi v8 기준 애플리케이션 초기화
    app = new PIXI.Application();
    await app.init({
      background: '#09090b',
      resizeTo: canvasContainer,
      antialias: true,
    });

    canvasContainer.appendChild(app.canvas);

    // 배경 분위기 표현용 그래픽 (예: 맵 구역 사각형)
    const bgGraphics = new PIXI.Graphics();
    bgGraphics.rect(0, 0, app.screen.width, app.screen.height);
    bgGraphics.fill({ color: 0x18181b });
    app.stage.addChild(bgGraphics);

    // 플레이어/캐릭터를 상징하는 그래픽 오브젝트 생성
    characterSprite = new PIXI.Graphics();
    characterSprite.circle(0, 0, 24);
    characterSprite.fill({ color: 0x38bdf8 }); // 파란색 원 (플레이어)
    characterSprite.stroke({ width: 3, color: 0xffffff });
    
    // 화면 중앙에 배치
    characterSprite.x = app.screen.width / 2;
    characterSprite.y = app.screen.height / 2;
    app.stage.addChild(characterSprite);

    // 간단한 부유 애니메이션 (Ticker 활용)
    let elapsed = 0;
    app.ticker.add((ticker) => {
      elapsed += ticker.deltaTime;
      if (characterSprite) {
        characterSprite.y = (app?.screen.height || 300) / 2 + Math.sin(elapsed * 0.05) * 6;
      }
    });
  });

  onDestroy(() => {
    if (app) {
      app.destroy(true, { children: true });
    }
  });

  // mapId나 locationId가 바뀔 때 시각 효과 업데이트 (반응성)
  $effect(() => {
    if (mapId && app) {
      console.log(`Pixi 캔버스 맵 갱신: ${mapId} / 장소: ${locationId}`);
    }
  });
</script>

<div class="pixi-wrapper" bind:this={canvasContainer}>
  <div class="canvas-overlay-info">
    <span class="badge">🗺️ 캔버스 뷰</span>
    <span class="coords">{mapId} : {locationId || '필드'}</span>
  </div>
</div>

<style>
  .pixi-wrapper {
    position: relative;
    width: 100%;
    height: 200px;
    background: #030712;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #334155;
  }

  .canvas-overlay-info {
    position: absolute;
    top: 8px;
    left: 8px;
    display: flex;
    gap: 0.5rem;
    align-items: center;
    pointer-events: none;
    z-index: 10;
  }

  .badge {
    background: rgba(15, 23, 42, 0.8);
    backdrop-filter: blur(4px);
    color: #38bdf8;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    font-weight: bold;
    border-radius: 4px;
    border: 1px solid #334155;
  }

  .coords {
    font-size: 0.75rem;
    color: #94a3b8;
    background: rgba(0, 0, 0, 0.5);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-family: monospace;
  }
</style>