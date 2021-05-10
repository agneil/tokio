use std::io;

#[cfg(windows)]
async fn windows_main() -> io::Result<()> {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::windows::{wait_named_pipe, NamedPipeClientOptions, NamedPipeOptions};
    use tokio::time;
    use winapi::shared::winerror;

    const PIPE_NAME: &str = r"\\.\pipe\named-pipe-multi-client";
    const N: usize = 10;

    // The first server needs to be constructed early so that clients can
    // be correctly connected. Otherwise a waiting client will error.
    //
    // Here we also make use of `first_pipe_instance`, which will ensure
    // that there are no other servers up and running already.
    let mut server = NamedPipeOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)?;

    let server = tokio::spawn(async move {
        // Artificial workload.
        time::sleep(Duration::from_secs(1)).await;

        for _ in 0..N {
            // Wait for client to connect.
            server.connect().await?;
            let mut inner = server;

            // Construct the next server to be connected before sending the one
            // we already have of onto a task. This ensures that the server
            // isn't closed (after it's done in the task) before a new one is
            // available. Otherwise the client might error with
            // `io::ErrorKind::NotFound`.
            server = NamedPipeOptions::new().create(PIPE_NAME)?;

            let _ = tokio::spawn(async move {
                let mut buf = [0u8; 4];
                inner.read_exact(&mut buf).await?;
                inner.write_all(b"pong").await?;
                Ok::<_, io::Error>(())
            });
        }

        Ok::<_, io::Error>(())
    });

    let mut clients = Vec::new();

    for _ in 0..N {
        clients.push(tokio::spawn(async move {
            let mut client = loop {
                match NamedPipeClientOptions::new().create(PIPE_NAME) {
                    Ok(client) => break client,
                    Err(e) if e.raw_os_error() == Some(winerror::ERROR_PIPE_BUSY as i32) => (),
                    Err(e) => return Err(e),
                }

                // This showcases a generic connect loop.
                //
                // We immediately try to create a client, if it's not found or
                // the pipe is busy we use the specialized wait function on the
                // client builder.
                wait_named_pipe(PIPE_NAME, Some(Duration::from_secs(5))).await?;
            };

            let mut buf = [0u8; 4];
            client.write_all(b"ping").await?;
            client.read_exact(&mut buf).await?;
            Ok::<_, io::Error>(buf)
        }));
    }

    for client in clients {
        let result = client.await?;
        assert_eq!(&result?[..], b"pong");
    }

    server.await??;
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(windows)]
    {
        windows_main().await?;
    }

    #[cfg(not(windows))]
    {
        println!("Named pipes are only supported on Windows!");
    }

    Ok(())
}
