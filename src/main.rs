use aci::*;

#[tokio::main]
async fn main()
{
    let (conn, listener,  mut events) = connect("127.0.0.1", 8765).await.unwrap();

    let h = tokio::spawn(listener);

    let login_resp = conn.a_auth("USERNAME", "PASSWORD").await.unwrap();

    if login_resp
    {
        println!("Successfully Authed");
        while let Some(event) = events.recv().await
        {
            println!("{:?}", event);
        }
    }

    tokio::join!(h);
}