use carapax::{async_trait, ErrorHandler, ErrorPolicy, HandlerError};

pub struct BotErrorHandler;

#[async_trait]
impl ErrorHandler for BotErrorHandler {
    async fn handle(&mut self, e: HandlerError) -> ErrorPolicy {
        eprintln!("{}", e);
        ErrorPolicy::Stop
    }
}
