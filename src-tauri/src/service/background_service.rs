//! Android 포그라운드 서비스 (CLAUDE.md 1-C-2).
//!
//! 화면이 꺼지거나 앱이 뒤로 가면 Android가 프로세스를 얼려 생성 중인 응답이 끊긴다.
//! 프론트(src/app/keepAlive.ts)가 AI 작업이 돌고 있는 동안만 서비스를 켜고, 작업이 없으면 끈다.
//! 서비스는 프로세스를 살려 두기만 한다 — 끌 때 작업을 멈추지 않는다
//! (예전에는 종료 신호에 실행 중 작업을 Interrupted로 바꿨는데, 그러면 서비스를 끄는 순간 생성이 끊길 수 있다.
//!  앱이 강제로 꺼지면 다음 시작 때 recover_on_startup이 정리한다).
use async_trait::async_trait;
use tauri::Runtime;
use tauri_plugin_background_service::{BackgroundService, ServiceContext, ServiceError};

pub struct AppBackgroundService;

impl AppBackgroundService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl<R: Runtime> BackgroundService<R> for AppBackgroundService {
    async fn init(&mut self, _ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        // 작업 복구(recover_on_startup)는 lib.rs setup에서 TaskManager 생성 직후 한 번 수행한다.
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        // 끄라는 신호가 올 때까지 기다리기만 한다 (작업은 TaskManager가 따로 돌린다)
        ctx.shutdown.cancelled().await;
        Ok(())
    }
}
