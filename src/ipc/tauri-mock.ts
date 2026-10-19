// src/mocks/tauri-mock.ts

// @tauri-apps/plugin-os 모킹
export const platform = () => 'windows';

// @tauri-apps/api/window 모킹
export const getCurrentWindow = () => ({
  label: 'main',
  minimize: () => {return},
  toggleMaximize: () => {return},
  close: () => {return},
});

export const Window = () => ({});
