pub mod search_index {
    tonic::include_proto!("searchindex");
}

use search_index::{
    indexer_client::IndexerClient, Entry, Query, Circle
};

use futures::stream;
use std::str;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = IndexerClient::connect("http://[::1]:50051").await?;

    println!("Adding entries");
    let request = tonic::Request::new(stream::iter(vec![
        Entry {
            data: r#"{"pepito": "hola"}"#.into(),
            geom: Some("SRID=4326; POINT(12 0)".into()),
            response: "response hola".as_bytes().into()
        },
        Entry {
            data: r#"{"key": "adeu"}"#.into(),
            geom: Some("SRID=4326; POINT(0 0)".into()),
            response: "response adéu".as_bytes().into()
        }
    ]));
    println!("Response: {:?}", client.add_entries(request).await?);


    println!("\n\nQuerying");
    let request = tonic::Request::new(Query {
        q: "hola".into(),
        radius: Some(Circle {
            lat: 11.5,
            long: 0.0,
            radius: 112_000.0
        })
    });
    let page = client.search(request).await?.into_inner();
    for response in page.responses {
        println!("{:?}", str::from_utf8(&response)?);
    }

    Ok(())
}
