use futures_util::StreamExt;
use futures_util::stream::SplitStream;

use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;

use serde_json::Value;

use chashmap::*;

use std::sync::Arc;

pub async fn listener(mut stream: SplitStream<WebSocketStream<TcpStream>>, open_requests: Arc<CHashMap<usize, tokio::sync::oneshot::Sender<Value>>>, event_channel: tokio::sync::mpsc::Sender<super::event::ACIEvent>)
{
    while let Some(message) = stream.next().await
    {
        if let Ok(msg) = message
        {
            let slice = msg.into_data();

            if let Ok(data) = serde_json::from_slice::<Value>(&slice)
            {
                if let Value::Object(mut obj) = data
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
                                    continue;
                                }
                            }
                        }
                    }

                    if obj.contains_key("cmd")
                    {
                        if let Value::String(s) = obj.remove("cmd").unwrap()
                        {
                            if s == "event"
                            {
                                let event_id = if obj.contains_key("event_id")
                                {
                                    obj.remove("event_id").unwrap()
                                }
                                else
                                {
                                    continue;
                                };

                                let data = if obj.contains_key("data")
                                {
                                    obj.remove("data").unwrap()
                                }
                                else
                                {
                                    continue;
                                }; 

                                let origin = if obj.contains_key("origin")
                                {
                                    if let Value::String(s) = obj.remove("origin").unwrap()
                                    {
                                        s
                                    }
                                    else
                                    {
                                        continue;
                                    }
                                }
                                else
                                {
                                    continue;
                                };
                                
                                let dest = if obj.contains_key("destination")
                                {
                                    if let Value::String(s) = obj.remove("destination").unwrap()
                                    {
                                        s
                                    }
                                    else
                                    {
                                        continue;
                                    }
                                }
                                else
                                {
                                    continue;
                                };

                                let event = super::event::ACIEvent::create(origin, dest, data, event_id);

                                event_channel.send(event).await.unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
