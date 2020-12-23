use futures_util::StreamExt;
use futures_util::stream::SplitStream;

use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;

use serde_json::Value;

use chashmap::*;

use std::sync::Arc;

pub async fn listener(stream: SplitStream<WebSocketStream<TcpStream>>, open_requests: Arc<CHashMap<usize, tokio::sync::oneshot::Sender<Value>>>)
{
    stream.for_each(|message| async
        {
            if let Ok(msg) = message
            {
                let slice = msg.into_data();

                if let Ok(data) = serde_json::from_slice::<Value>(&slice)
                {
                    if let Value::Object(obj) = data
                    {
                        if obj.contains_key("unique_id")
                        {
                            if let Value::Number(num) = obj.get("unique_id").unwrap()
                            {
                                if num.is_u64()
                                {
                                    let num = num.as_u64().unwrap() as usize;
                                    if open_requests.contains_key(&num)
                                    {
                                        let tx = open_requests.remove(&num).unwrap();
                                        tx.send(Value::Object(obj)).unwrap();
                                    }
                                }
                            }
                        }
                    }
                }
            }
    }).await;
}
