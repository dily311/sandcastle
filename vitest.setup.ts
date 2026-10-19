// vitest.setup.ts
import { vi } from 'vitest';
// vitest.setup.ts 상단에 추가
if (typeof window !== 'undefined') {
  (window as any).__TAURI_INTERNALS__ = {};
}

// 1. 네임스페이스 전체 및 개별 export를 모두 대응하도록 수정
vi.mock('@tauri-apps/plugin-os', () => ({
  platform: vi.fn(() => 'windows'),
  default: {
    platform: vi.fn(() => 'windows'),
  }
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    label: 'main',
  })),
  default: {
    getCurrentWindow: vi.fn(() => ({
      label: 'main',
    })),
  }
}));
