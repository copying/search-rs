mod postgres;
mod search_index;
mod statements;
mod utils;

use futures::future;
use futures_util::stream::StreamExt;
use rand::Rng;
use tokio_postgres::{Client, Error};
use tonic::{transport::Server, Request, Response, Status, Code};

use postgres::make_postgres_client;
use search_index::{
    indexer_server::{Indexer, IndexerServer},
    Index, IndexId, Entry, Query, Page
};
use utils::{internal_error, require_arg};

pub struct SearchIndex {
    client : Client,
}

const SCHEMA: &str = "index";
const INDEX_TABLE: &str = "index";


async fn set_entries(
    client: &mut Client,
    tmp_name: &str,
    request: Request<tonic::Streaming<Entry>>
) -> Result<(), Status> {
    let stream = request.into_inner();
    futures::pin_mut!(stream);

    let transaction = client.transaction().await.map_err(internal_error)?;
    let query: &str = &statements::create_index_table(&SCHEMA, &tmp_name);
    transaction.execute(query, &[]).await.map_err(internal_error)?;

    let query: &str = &statements::add_entry(&SCHEMA, &tmp_name);
    let stmt = transaction.prepare(query).await.map_err(internal_error)?;
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let data = entry.data;
        let geom = entry.geom;
        let response = entry.response;
        transaction.execute(&stmt, &[&data, &geom, &response]).await.map_err(internal_error)?;
    }

    transaction.commit().await.map_err(internal_error)?;
    Ok(())
}

async fn swap_tables(
    client: &mut Client,
    name: &str,
    from_table: &str
) -> Result<(), Error> {
    let temp_table = format!("{}_", from_table);
    {
        let transaction = client.transaction().await?;
        let query: &str = &statements::rename_table(&SCHEMA, &name, &temp_table);
        transaction.execute(query, &[]).await?;
        let query: &str = &statements::rename_table(&SCHEMA, &from_table, &name);
        transaction.execute(query, &[]).await?;
        transaction.commit().await?;
    }
    let query: &str = &statements::drop_table(&SCHEMA, &temp_table);
    client.execute(query, &[]).await?;

    Ok(())
}


#[tonic::async_trait]
impl Indexer for SearchIndex {
    async fn delete_index(
        &self,
        request: Request<IndexId>
    ) -> Result<Response<()>, Status> {
        let name = request.into_inner().name;
        if name.is_empty() {
            return Err(Status::new(Code::InvalidArgument, "The name connot be empty"))
        } else if name == INDEX_TABLE {
            return Err(Status::new(Code::InvalidArgument, "This name is already taken by the main table"))
        }

        let mut client = make_postgres_client().await.map_err(internal_error)?;
        {
            let transaction = client.transaction().await.map_err(internal_error)?;
            let query: &str = &statements::drop_table(&SCHEMA, &name);
            transaction.execute(query, &[]).await.map_err(internal_error)?;
            let query: &str = &statements::delete_index(&SCHEMA, &INDEX_TABLE);
            transaction.execute(query, &[&name]).await.map_err(internal_error)?;
            transaction.commit().await.map_err(internal_error)?;
        }
        Ok(Response::new(()))
    }

    async fn add_index(
        &self,
        request: Request<Index>
    ) -> Result<Response<()>, Status> {
        let index_def = request.into_inner();
        if index_def.name.is_empty() {
            return Err(Status::new(Code::InvalidArgument, "The name connot be empty"))
        } else if index_def.name == INDEX_TABLE {
            return Err(Status::new(Code::InvalidArgument, "This name is already taken by the main table"))
        }

        if index_def.response_size <= 0 {
            return Err(Status::new(Code::InvalidArgument, "Response size must be a positive integer"))
        }

        let mut client = make_postgres_client().await.map_err(internal_error)?;
        {
            let transaction = client.transaction().await.map_err(internal_error)?;
            let query: &str = &statements::add_index(&SCHEMA, &INDEX_TABLE);
            let result = transaction.execute(query, &[&index_def.name, &index_def.language, &index_def.response_size]).await;
            result.map_err(|e| {
                match e.code().map(|code| code.code()) {
                    Some("23505") => Status::new(Code::InvalidArgument, "Index already exists"),
                    _ => {
                        eprintln!("{}", e);
                        Status::new(Code::Internal, "Internal error")
                    }
                }
            })?;
            let query: &str = &statements::create_index_table(&SCHEMA, &index_def.name);
            transaction.execute(query, &[]).await.map_err(internal_error)?;
            transaction.commit().await.map_err(internal_error)?;
        }
        Ok(Response::new(()))
    }


    async fn set_entries(
        &self,
        request: Request<tonic::Streaming<Entry>>
    ) -> Result<Response<()>, Status> {
        let metadata = request.metadata();
        let name = require_arg(metadata.get("x-index-name"))?;
        let query: &str = &statements::get_index(&SCHEMA, &INDEX_TABLE);
        let results = self.client.query(query, &[&name]).await.map_err(internal_error)?;
        if results.len() < 1 {
            return Err(Status::new(Code::InvalidArgument, "The selected index doesn't exist"))
        }

        let tmp_name: &str = &{
            let mut rng = rand::thread_rng();
            format!("_{}_{}", name, rng.gen::<u32>())
        };

        let mut client = make_postgres_client().await.map_err(internal_error)?;
        set_entries(&mut client, &tmp_name, request).await?;
        swap_tables(&mut client, &name, &tmp_name).await.map_err(internal_error)?;
        Ok(Response::new(()))
    }


    async fn search(
        &self,
        request: Request<Query>
    ) -> Result<Response<Page>, Status> {
        let metadata = request.metadata();
        let name = require_arg(metadata.get("x-index-name"))?;

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

        let query: &str = &statements::get_index(&SCHEMA, &INDEX_TABLE);
        let results = self.client.query(query, &[&name]).await.map_err(internal_error)?;
        if results.len() < 1 {
            return Err(Status::new(Code::InvalidArgument, "The selected index doesn't exist"))
        }
        let lang: &str = results[0].get(0);
        let response_size: i32 = results[0].get(1);

        let query: &str = &statements::search(&SCHEMA, &name);
        let results = self.client.query(query, &[&q.q, &lat, &lng, &radius, &lang, &response_size]).await.map_err(internal_error)?;

        Ok(Response::new(Page {
            responses: results.into_iter().map(|row| row.get(0)).collect()
        }))
    }
}

async fn ensure_data_structure(client: &Client) -> Result<(), Error> {
    future::try_join(
        client.execute(statements::ADD_POSTGIS, &[]),
        client.execute(&statements::create_main_table(&SCHEMA, &INDEX_TABLE) as &str, &[]),
    ).await?;

    // TODO: Add more checks for the data structure

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = make_postgres_client().await?;

    ensure_data_structure(&client).await?;

    let search_index = SearchIndex { client: client };

    let addr = "[::1]:3009".parse()?;

    println!("Server configured");
    Server::builder()
        .add_service(IndexerServer::new(search_index))
        .serve(addr)
        .await?;


    Ok(())
}
