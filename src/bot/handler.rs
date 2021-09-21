use super::Context;
use carapax::{handler, types::Message, ExecuteError, HandlerResult};

#[handler]
pub async fn message_handler(
    context: &Context,
    message: Message,
) -> Result<HandlerResult, ExecuteError> {
    if let Some(text) = message.get_text() {
        println!("here2");
        context.sender.send(text.data.clone()).await.unwrap();
    }
    Ok(HandlerResult::Stop)
}
