use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::sync::{watch, Mutex as AsyncMutex};

pub struct ChatRequest {
    pub id: String,
    pub character_id: String,
    pub session_id: String,
    pub progress: Mutex<String>,
    cancel: watch::Sender<bool>,
    finished: watch::Sender<bool>,
}

impl ChatRequest {
    pub fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }

    pub async fn cancelled(&self) {
        let mut receiver = self.cancel.subscribe();
        let _ = receiver.wait_for(|cancelled| *cancelled).await;
    }

    pub fn append(&self, text: &str) {
        self.progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push_str(text);
    }

    pub fn partial(&self) -> String {
        self.progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

#[derive(Default)]
struct RequestState {
    active: Option<Arc<ChatRequest>>,
    early_cancellations: VecDeque<String>,
}

#[derive(Default)]
pub struct RequestManager(AsyncMutex<RequestState>);

impl RequestManager {
    pub async fn begin(
        &self,
        id: String,
        character_id: String,
        session_id: String,
    ) -> Result<Arc<ChatRequest>, String> {
        let mut state = self.0.lock().await;
        if let Some(index) = state
            .early_cancellations
            .iter()
            .position(|item| item == &id)
        {
            state.early_cancellations.remove(index);
            return Err("请求已取消。".to_string());
        }
        if state.active.is_some() {
            return Err("请等待当前回复结束，或先停止生成。".to_string());
        }
        let (cancel, _) = watch::channel(false);
        let (finished, _) = watch::channel(false);
        let request = Arc::new(ChatRequest {
            id,
            character_id,
            session_id,
            progress: Mutex::new(String::new()),
            cancel,
            finished,
        });
        state.active = Some(request.clone());
        Ok(request)
    }

    pub async fn cancel(&self, id: &str) {
        let request = {
            let mut state = self.0.lock().await;
            match state
                .active
                .as_ref()
                .filter(|request| request.id == id)
                .cloned()
            {
                Some(request) => {
                    request.cancel.send_replace(true);
                    Some(request)
                }
                None => {
                    if !state.early_cancellations.iter().any(|item| item == id) {
                        state.early_cancellations.push_back(id.to_string());
                        if state.early_cancellations.len() > 128 {
                            state.early_cancellations.pop_front();
                        }
                    }
                    None
                }
            }
        };
        if let Some(request) = request {
            let mut finished = request.finished.subscribe();
            let _ = finished.wait_for(|finished| *finished).await;
        }
    }

    pub async fn finish(&self, request: &Arc<ChatRequest>) {
        let mut state = self.0.lock().await;
        if state
            .active
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, request))
        {
            state.active = None;
        }
        request.finished.send_replace(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_waits_for_finalization_and_releases_request() {
        let manager = Arc::new(RequestManager::default());
        let request = manager
            .begin("a".into(), "character-a".into(), "session-a".into())
            .await
            .unwrap();
        assert!(manager
            .begin("b".into(), "character-b".into(), "session-b".into())
            .await
            .is_err());
        let cancel = tokio::spawn({
            let manager = manager.clone();
            async move { manager.cancel("a").await }
        });
        request.cancelled().await;
        assert!(request.is_cancelled());
        assert!(!cancel.is_finished());
        manager.finish(&request).await;
        cancel.await.unwrap();
        assert!(manager
            .begin("b".into(), "character-b".into(), "session-b".into())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn early_cancel_cannot_turn_into_a_late_network_request() {
        let manager = RequestManager::default();
        manager.cancel("a").await;
        assert!(manager
            .begin("a".into(), "character-a".into(), "session-a".into())
            .await
            .is_err());
    }
}
