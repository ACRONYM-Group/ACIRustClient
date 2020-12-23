use futures_util::{StreamExt, SinkExt};
use futures_util::stream::SplitSink;

use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::protocol::Message};

use serde_json::{json, Value};

use chashmap::*;

use std::sync::Arc;

use tokio::sync::Mutex;

use super::errors::*;
use super::listener::listener;

/// Connection to the ACI server
#[derive(Debug)]
pub struct Connection
{
    pub sink: Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>,
    pub open_requests: Arc<CHashMap<usize, tokio::sync::oneshot::Sender<Value>>>,
    pub unique_id: std::sync::atomic::AtomicUsize,
    pub event_id: std::sync::atomic::AtomicUsize
}

impl Connection
{
    fn new(sink: SplitSink<WebSocketStream<TcpStream>, Message>, open_requests: Arc<CHashMap<usize, tokio::sync::oneshot::Sender<Value>>>) -> Self
    {
        Self{sink: Mutex::new(sink), open_requests, unique_id: std::sync::atomic::AtomicUsize::new(0), event_id: std::sync::atomic::AtomicUsize::new(0)}
    }

    /// Get the next event_id
    fn next_id(&self, origin: &str, dest: &str) -> String
    {
        let num = self.event_id.fetch_add(1, std::sync::atomic::Ordering::Release);
        format!("EVENT_{}_{}->{}", num, origin, dest)
    }

    /// Close the connection to the ACI Server
    pub async fn close(&self) -> Result<(), ACIClientError>
    {
        let mut sink = self.sink.lock().await;
        
        match sink.close().await
        {
            Ok(()) => Ok(()),
            Err(e) => 
            {
                Err(ACIClientError::new(format!("{}", e)))
            }
        }
    }

    /// Send a raw packet to the ACI Server
    pub async fn send_packet(&self, data: Value) -> Result<Value, ACIError>
    {
        if let Value::Object(mut obj) = data
        {
            let num = self.unique_id.fetch_add(1, std::sync::atomic::Ordering::Release);
            obj.insert("unique_id".to_string(), Value::Number(serde_json::Number::from(num)));

            let (tx, rx) = tokio::sync::oneshot::channel();
            self.open_requests.insert(num, tx);

            {
                let mut sink = self.sink.lock().await;

                match sink.send(Message::Text(Value::Object(obj).to_string())).await
                {
                    Ok(_) => {},
                    Err(e) =>
                    {
                        return Err(ACIError::ClientError(ACIClientError::new(format!("Cannot write to socket {}", e))))
                    }
                }

            }   

            tokio::spawn(async move {
                let packet = rx.await.unwrap();

                if let Value::Object(obj) = &packet
                {
                    if obj.contains_key("mode")
                    {
                        if let Value::String(mode) = obj.get("mode").unwrap()
                        {
                            if mode == "error"
                            {
                                let msg = if obj.contains_key("msg")
                                {
                                    if let Value::String(msg) = obj.get("msg").unwrap()
                                    {
                                        msg.clone()
                                    }
                                    else
                                    {
                                        "Unknown Error: Malformed msg field".to_string()
                                    }
                                }
                                else
                                {
                                    "Unknown Error: No msg field".to_string()
                                };

                                return Err(ACIError::ServerError(ACIServerError::new(msg)))
                            }
                        }
                        else
                        {
                            return Err(ACIError::ServerError(ACIServerError::new("Malformed packet, `mode` field is not a string".to_string())))
                        }
                    }
                    else
                    {
                        return Err(ACIError::ServerError(ACIServerError::new("Malformed packet, no `mode` field".to_string())))
                    }
                }

                Ok(packet)
            }).await.unwrap()
            }
        else
        {
            Err(ACIError::ClientError(ACIClientError::new("Bad JSON Packet".to_string())))
        }
            
    }

    /// Authenticate using acronym authentication on the server
    pub async fn a_auth(&self, id: &str, token: &str) -> Result<bool, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "a_auth", "id": id, "token": token})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("msg")
                        {
                            if let Value::String(msg) = obj.get("msg").unwrap()
                            {
                                return Ok(msg == "success")
                            }
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }    

    /// Read a database from disk on the server
    pub async fn read_from_disk(&self, db_key: &str) -> Result<(), ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "read_from_disk", "db_key": db_key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Write a database to disk on the server
    pub async fn write_to_disk(&self, db_key: &str) -> Result<(), ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "write_to_disk", "db_key": db_key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get a vector of keys in a loaded database on the server
    pub async fn list_keys(&self, db_key: &str) -> Result<Vec<String>, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "list_keys", "db_key": db_key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("val")
                        {
                            if let Value::Array(array) = obj.get("val").unwrap()
                            {
                                let mut result = vec![];

                                for v in array
                                {
                                    if let Value::String(s) = v
                                    {
                                        result.push(s.clone())
                                    }
                                    else
                                    {
                                        return Err(ACIError::ClientError(ACIClientError::new(format!("Key is not a string, `{}`", v))))
                                    }
                                }

                                return Ok(result)
                            }
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get a vector of database keys loaded on the server
    pub async fn list_databases(&self) -> Result<Vec<String>, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "list_keys"})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("val")
                        {
                            if let Value::Array(array) = obj.get("val").unwrap()
                            {
                                let mut result = vec![];

                                for v in array
                                {
                                    if let Value::String(s) = v
                                    {
                                        result.push(s.clone())
                                    }
                                    else
                                    {
                                        return Err(ACIError::ClientError(ACIClientError::new(format!("Key is not a string, `{}`", v))))
                                    }
                                }

                                return Ok(result)
                            }
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get a value for a key in the database
    pub async fn get_value(&self, db_key: &str, key: &str) -> Result<Value, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "get_value", "db_key": db_key, "key": key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("val")
                        {
                            return Ok(obj.get("val").unwrap().clone());
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Write a value to a key in the database
    pub async fn set_value(&self, db_key: &str, key: &str, data: Value) -> Result<(), ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "set_value", "db_key": db_key, "key": key, "val": data})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get a value at an index for a key in the database
    pub async fn get_index(&self, db_key: &str, key: &str, index: usize) -> Result<Value, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "get_index", "db_key": db_key, "key": key, "index": index})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("val")
                        {
                            return Ok(obj.get("val").unwrap().clone());
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Write a value at an index to a key in the database
    pub async fn set_index(&self, db_key: &str, key: &str, index: usize, data: Value) -> Result<(), ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "set_index", "db_key": db_key, "key": key, "index": index, "val": data})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Append a value to an array stored in a key in the database
    pub async fn append_list(&self, db_key: &str, key: &str, data: Value) -> Result<usize, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "append_list", "db_key": db_key, "key": key, "val": data})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("next")
                        {
                            if let Value::Number(num) = obj.get("next").unwrap()
                            {
                                if let Some(index) = num.as_u64()
                                {
                                    return Ok(index as usize)
                                }
                            }
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get the length of a list on the server
    pub async fn get_list_length(&self, db_key: &str, key: &str) -> Result<usize, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "get_list_length", "db_key": db_key, "key": key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("length")
                        {
                            if let Value::Number(num) = obj.get("length").unwrap()
                            {
                                if let Some(index) = num.as_u64()
                                {
                                    return Ok(index as usize)
                                }
                            }
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Get the last n values (or the entirety) of an array stored in a key on the server  
    pub async fn get_recent(&self, db_key: &str, key: &str, n: usize) -> Result<Value, ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "get_index", "db_key": db_key, "key": key, "num": n})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        if obj.contains_key("val")
                        {
                            return Ok(obj.get("val").unwrap().clone());
                        }
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Create a database on the server with the given key
    pub async fn create_database(&self, db_key: &str) -> Result<(), ACIError>
    {
        let resp = self.send_packet(json!({"cmd": "create_database", "db_key": db_key})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ok"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }

    /// Send an event to the server
    pub async fn send_event(&self, origin: &str, dest: &str, data: Value) -> Result<(), ACIError>
    {
        let id = self.next_id(origin, dest);

        let resp = self.send_packet(json!({"cmd": "event", "event_id": id, "destination": dest, "origin": origin, "data": data})).await?;

        if let Value::Object(obj) = &resp
        {
            if obj.contains_key("mode")
            {
                if let Value::String(mode) = obj.get("mode").unwrap()
                {
                    if mode == "ack"
                    {
                        return Ok(())
                    }  
                }
            }
        }
        
        Err(ACIError::ClientError(ACIClientError::new(format!("Response is malformed, `{}`", resp))))
    }
}

/// Connect to an ACI server at the given ip and port
pub async fn connect(ip: &str, port: usize) -> Result<(Connection, impl std::future::Future<Output=()>), ACIError>
{
    let url_str = format!("ws://{}:{}", ip, port);
    let url = match url::Url::parse(&url_str)
    {
        Ok(url) => url,
        Err(e) =>
        {
            return Err(ACIError::ClientError(ACIClientError::new(format!("Cannot create url for `{}` {}", url_str, e))))
        }
    };
    let (stream, _) = connect_async(url).await.expect("Unable to connect");

    let (sink, stream) = stream.split();
    
    let open_requests = Arc::new(CHashMap::new());
    let conn = Connection::new(sink, open_requests.clone());

    Ok((conn, listener(stream, open_requests)))
}