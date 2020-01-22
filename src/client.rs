pub mod search_index {
    tonic::include_proto!("searchindex");
}

use search_index::{
    indexer_client::IndexerClient, Entry
};

use futures::stream;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = IndexerClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(stream::iter(vec![
        Entry {
            data: r#"{"pepito": "a"}"#.into(),
            geom: None,
            response: "response".as_bytes().into()
        },
        Entry {
            data: r#"{"key": "b"}"#.into(),
            geom: None,
            response: "response".as_bytes().into()
        }
    ]));

    println!("Response: {:?}", client.add_entries(request).await?);
    Ok(())
}
