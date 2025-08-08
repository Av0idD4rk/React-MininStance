mod error;

use common::init_logging;
use tracing::{info, error};
use scheduler_service::run;
use crate::error::SchedulerError;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_logging();
    info!("Scheduler starting up");

    if let Err(e) = run().await {
        error!("Scheduler failed: {}", e);
        std::process::exit(1);
    }
}
