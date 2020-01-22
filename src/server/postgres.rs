use tokio_postgres::{Client, NoTls, Error};

pub async fn make_postgres_client() -> Result<Client, Error> {
    let (client, connection) =
        tokio_postgres::connect("host=/var/run/postgresql user=copying dbname=index_test", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}
