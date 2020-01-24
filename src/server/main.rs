mod postgres;
mod search_index;
mod statements;
mod utils;

use futures_util::stream::StreamExt;
use tokio_postgres::Client;
use tonic::{transport::Server, Request, Response, Status, Code};

use postgres::make_postgres_client;
use search_index::{
    indexer_server::{Indexer, IndexerServer},
    Entry, Query, Page
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

    async fn search(
        &self,
        request: Request<Query>
    ) -> Result<Response<Page>, Status> {
        let q = request.into_inner();

        let (lat, lng, radius) = match q.radius {
            Some(circle) => {
                if !(circle.lat >= -90.0 && circle.lat <= 90.0) {
                    return Err(Status::new(Code::InvalidArgument, "Latitude is outside the valid range"))
                }
                if !(circle.lat > -180.0 && circle.lat <= 90.0) {
                    return Err(Status::new(Code::InvalidArgument, "Longitude is outside the valid range"))
                }
                if !circle.radius.is_finite() {
                    return Err(Status::new(Code::InvalidArgument, "Radius are not a finite number"))
                }
                (Some(circle.lat), Some(circle.long), Some(circle.radius))
            },
            None => (None, None, None)
        };

        let results = err_status(self.client.query(statements::SEARCH, &[&q.q, &lat, &lng, &radius]).await)?;


        Ok(Response::new(Page {
            responses: results.into_iter().map(|row| row.get(0)).collect()
        }))
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
