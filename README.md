# AI Game Chat — scalable MVP

Stack: Tauri 2 + Svelte 5 + Vite + TypeScript + Pure CSS + PixiJS 8 + Rust/reqwest.

## Product boundary
- Local-first desktop/mobile application.
- No application server.
- Rust owns provider HTTP calls so API keys do not live in the Svelte/WebView networking layer.
- Chat mode is the primary product.
- Game mode is a separate runtime that can consume the same conversation/state layer.
- **View more details**: [Tech spec](./docs/Tech%20Spec.md)

## Next production steps
1. Add SQLite persistence through Tauri SQL plugin or a Rust SQLite layer.
2. Move API keys to OS secure storage (Keychain/Keystore/Credential Manager).
3. Add provider adapters instead of hard-coding OpenAI-compatible responses.
4. Add streaming events from Rust -> Svelte.
5. Add migrations, repository interfaces, conversation/message persistence.
6. Add game manifest + scene/state schema and a sandboxed asset loader.
7. Add import/export of characters, presets and games.
8. Add Android-specific permissions, lifecycle and secure storage implementation.
9. Add tests for domain, provider adapters, migrations and game state transitions.

## Run
npm install
npm run tauri:dev

For Android, initialize the Tauri Android target and use the Tauri CLI Android commands appropriate to your installed SDK/NDK.

## License
This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)** - see the [LICENSE](LICENSE) file for details.

요약: 개조 재배포 다 가능. 출처와 소스코드 공개 필수
다만 개조 재배포 시 문제 책임 소재는 해당 프로젝트에 없음
개조버전 오류에 대한 문의의 경우 이곳에 남기지 말것
