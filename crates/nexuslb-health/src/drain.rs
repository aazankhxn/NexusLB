use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

use nexuslb_core::backend::Backend;
use nexuslb_core::types::BackendState;

pub struct DrainController;

impl DrainController {
    /// Initiate graceful backend draining with a timeout
    pub async fn drain_backend(backend: Arc<Backend>, timeout: Duration) -> bool {
        info!(
            backend_id = %backend.id(),
            name = %backend.name(),
            timeout_secs = timeout.as_secs(),
            active_connections = backend.active_connections(),
            "Starting graceful backend draining"
        );

        backend.set_state(BackendState::Draining);

        let start = Instant::now();
        while start.elapsed() < timeout {
            if backend.active_connections() == 0 {
                backend.set_state(BackendState::Down);
                info!(
                    backend_id = %backend.id(),
                    name = %backend.name(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "Backend drained cleanly (0 active connections) -> DOWN"
                );
                return true;
            }
            sleep(Duration::from_millis(100)).await;
        }

        backend.set_state(BackendState::Down);
        warn!(
            backend_id = %backend.id(),
            name = %backend.name(),
            remaining_connections = backend.active_connections(),
            "Drain timeout reached before all connections finished -> forced DOWN"
        );
        false
    }
}
