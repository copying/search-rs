mod postgres;
mod search_index;
mod statements;
mod utils;

use futures_util::stream::StreamExt;
use rand::Rng;
use tokio_postgres::{Client};
use tonic::{transport::Server, Request, Response, Status, Code};

use postgres::make_postgres_client;
use search_index::{
    indexer_server::{Indexer, IndexerServer},
    Entry, Query, Page
};
use utils::{err_status, require_arg};

pub struct SearchIndex {
    client : Client,
}

const SCHEMA: &str = "index";


async fn set_entries(
    client: &mut Client,
    tmp_name: &str,
    request: Request<tonic::Streaming<Entry>>
) -> Result<(), Status> {
    let stream = request.into_inner();
    futures::pin_mut!(stream);

    let transaction = err_status(client.transaction().await)?;
    let query: &str = &statements::create_index_table(&SCHEMA, &tmp_name);
    err_status(transaction.execute(query, &[]).await)?;

    let query: &str = &statements::add_entry(&SCHEMA, &tmp_name);
    let stmt = err_status(transaction.prepare(query).await)?;
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let data = entry.data;
        let geom = entry.geom;
        let response = entry.response;
        err_status(transaction.execute(&stmt, &[&data, &geom, &response]).await)?;
    }

    err_status(transaction.commit().await)?;
    Ok(())
}

async fn swap_tables(
    client: &mut Client,
    name: &str,
    from_table: &str
) -> Result<(), Status> {
    let temp_table = format!("{}_", from_table);
    {
        let transaction = err_status(client.transaction().await)?;
        let query: &str = &statements::rename_table(&SCHEMA, &name, &temp_table);
        err_status(transaction.execute(query, &[]).await)?;
        let query: &str = &statements::rename_table(&SCHEMA, &from_table, &name);
        err_status(transaction.execute(query, &[]).await)?;
        err_status(transaction.commit().await)?;
    }
    let query: &str = &statements::drop_table(&SCHEMA, &temp_table);
    err_status(client.execute(query, &[]).await)?;

    Ok(())
}


#[tonic::async_trait]
impl Indexer for SearchIndex {
    async fn set_entries(
        &self,
        request: Request<tonic::Streaming<Entry>>
    ) -> Result<Response<()>, Status> {
        let name: &str = "test";
        let tmp_name: &str = &{
            let mut rng = rand::thread_rng();
            format!("_{}_{}", name, rng.gen::<u32>())
        };

        let mut client = err_status(make_postgres_client().await)?;
        set_entries(&mut client, &tmp_name, request).await?;
        swap_tables(&mut client, &name, &tmp_name).await?;
        Ok(Response::new(()))
    }

    async fn search(
        &self,
        request: Request<Query>
    ) -> Result<Response<Page>, Status> {
        let metadata = request.metadata();
        let table_name = require_arg(metadata.get("x-index-name"))?;

        let q = request.into_inner();

        let (lat, lng, radius) = match q.radius {
            Some(circle) => {
                if !(circle.lat >= -90.0 && circle.lat <= 90.0) {
                    return Err(Status::new(Code::InvalidArgument, "Latitude is outside the valid range"))
                }
                if !(circle.lat > -180.0 && circle.lat <= 180.0) {
                    return Err(Status::new(Code::InvalidArgument, "Longitude is outside the valid range"))
                }
                if !circle.radius.is_finite() {
                    return Err(Status::new(Code::InvalidArgument, "Radius are not a finite number"))
                }
                (Some(circle.lat), Some(circle.long), Some(circle.radius))
            },
            None => (None, None, None)
        };

        let query: &str = &statements::search(&SCHEMA, &table_name);
        let results = err_status(self.client.query(query, &[&q.q, &lat, &lng, &radius]).await)?;


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
