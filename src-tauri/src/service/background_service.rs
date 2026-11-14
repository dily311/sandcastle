use async_trait::async_trait;
use std::sync::Arc;
use tauri::{Manager, Runtime};
use tauri_plugin_background_service::{BackgroundService, ServiceContext, ServiceError};

use crate::task::TaskManager;

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
        // 서비스 종료 신호가 오면 실행 중인 작업을 Interrupted로 멈춘다.
        if let Some(manager) = ctx.app.try_state::<Arc<TaskManager>>() {
            let manager = manager.inner().clone();
            let _ = manager.run_worker(ctx.shutdown.clone()).await;
        }
        Ok(())
    }
}
