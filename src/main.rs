use acilib::*;

use std::sync::Arc;

#[tokio::main]
async fn main()
{
    let (conn, listener) = connect("127.0.0.1", 8765).await.unwrap();

    let h = tokio::spawn(listener);

    let login_resp = conn.a_auth("USERNAME", "PASSWORD").await.unwrap();

    let c = Arc::new(conn);

    if login_resp
    {
        let conn = c.clone();

        conn.read_from_disk("test").await.unwrap();

        conn.set_value("test", "value0", serde_json::json!("Hello World")).await.unwrap();

       tokio::spawn(reader(conn.clone(), "read0"));
       tokio::spawn(writer(conn.clone()));
       tokio::spawn(reader(conn.clone(), "read1"));
       tokio::spawn(reader(conn.clone(), "read2"));
    }

    tokio::join!(h);
}

async fn writer(conn: Arc<acilib::Connection>)
{
    for v in 0..
    {
        conn.set_value("test", "value0", serde_json::json!(v)).await.unwrap();
    }
}

async fn reader(conn: Arc<acilib::Connection>, name: &str)
{
    loop
    {
        println!("Value({}): {}", name, conn.get_value("test", "value0").await.unwrap());
    }
}