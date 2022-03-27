use tokio_stream::Stream;

use crate::model::Document;

mod static_list;
mod fs;

trait DocumentSource<T: Stream<Item=anyhow::Result<Document>>> {
    fn fetch(self) -> T;
}