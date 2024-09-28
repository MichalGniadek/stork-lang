mod server;

use async_std::{io, net::TcpListener, stream::StreamExt as _, task};

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:50022").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            task::spawn(async move {
                server::spawn(&stream).await;
            });
        }
        io::Result::<()>::Ok(())
    })
}
