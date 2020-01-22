mod postgres;
mod search_index;
mod statements;
mod utils;

use futures_util::stream::StreamExt;
use tokio_postgres::Client;
use tonic::{transport::Server, Request, Response, Status};

use postgres::make_postgres_client;
use search_index::{
    indexer_server::{Indexer, IndexerServer},
    Entry
};
use utils::err_status;

pub struct SearchIndex {
    client : Client,
}

#[tonic::async_trait]
impl Indexer for SearchIndex {
    async fn add_entries(
        &self,
        request: Request<tonic::Streaming<Entry>>
    ) -> Result<Response<()>, Status> {
        let stream = request.into_inner();
        futures::pin_mut!(stream);

        let stmt = err_status(self.client.prepare(statements::ADD_ENTRY).await)?;
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            let data = entry.data;
            let geom = entry.geom;
            let response = entry.response;
            err_status(self.client.execute(&stmt, &[&data, &geom, &response]).await)?;
        }
        Ok(Response::new(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = make_postgres_client().await?;

    // ensure_data_structure(&client).await?;

    let search_index = SearchIndex { client: client };

    let addr = "[::1]:50051".parse()?;

    println!("Server configured");
    Server::builder()
        .add_service(IndexerServer::new(search_index))
        .serve(addr)
        .await?;


    Ok(())
}
