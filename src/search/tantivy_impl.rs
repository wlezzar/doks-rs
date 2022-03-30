use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tantivy::{doc, Index, IndexReader, IndexWriter};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, SchemaBuilder, STORED, STRING, TEXT};

use crate::model::Document;
use crate::search::SearchEngine;

struct TantivySearchEngine {
    index: Index,
    writer: Arc<RwLock<IndexWriter>>,
    reader: IndexReader,
    fields: SchemaFields,
    options: Options,
}

struct Options {
    default_fields: Vec<Field>,
}

#[derive(Clone)]
struct SchemaFields {
    id: Field,
    title: Field,
    link: Field,
    content: Field,
    source: Field,
}

impl TantivySearchEngine {
    fn new<T: AsRef<Path>>(path: T) -> anyhow::Result<Self> {
        let path = path.as_ref();

        match path.parent() {
            Some(parent) if !parent.exists() => std::fs::create_dir_all(path)?,
            _ => {}
        }

        let mut schema_builder = SchemaBuilder::new();
        let id = schema_builder.add_text_field("id", STRING | STORED);
        let title = schema_builder.add_text_field("title", TEXT | STORED);
        let link = schema_builder.add_text_field("link", STRING | STORED);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let source = schema_builder.add_text_field("source", STRING | STORED);

        let default_fields = vec![title.clone(), content.clone()];
        let fields = SchemaFields { title, id, link, content, source };

        let schema = schema_builder.build();
        let index = Index::open_or_create(
            MmapDirectory::open(path)?,
            schema.clone(),
        )?;


        let reader = index.reader()?;
        let writer = Arc::new(RwLock::new(index.writer(50_000_000)?));

        Ok(Self { index, writer, reader, fields, options: Options { default_fields } })
    }
}

#[async_trait]
impl SearchEngine<Pin<Box<dyn tokio_stream::Stream<Item=Document>>>> for TantivySearchEngine {
    async fn index<I: IntoIterator<Item=Document> + Send + 'static>(&self, documents: I) -> anyhow::Result<()> {
        let writer = self.writer.clone();
        let fields = self.fields.clone();

        let task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let documents = documents.into_iter();
            for document in documents {
                writer.read().unwrap().add_document(doc!(
                    fields.title => document.title,
                    fields.id => document.id,
                    fields.content => document.content,
                    fields.link => document.link,
                    fields.source => document.source,
                ));
            }

            writer.write().unwrap().commit()?;

            Ok(())
        });

        task.await?
    }

    fn search(&self, query: String) -> anyhow::Result<Pin<Box<dyn tokio_stream::Stream<Item=Document>>>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            self.options.default_fields.clone(),
        );
        let query = query_parser.parse_query(query.as_str())?;
        let top_docs = searcher.search(query.borrow(), &TopDocs::with_limit(10))?;

        let mut results = Vec::new();

        for (_, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let converted = Document {
                title: doc.get_first(self.fields.title)
                    .and_then(|f| f.text())
                    .expect("Field title of type text not found")
                    .to_string(),
                id: doc.get_first(self.fields.id)
                    .and_then(|f| f.text())
                    .expect("Field id of type text not found")
                    .to_string(),
                link: doc.get_first(self.fields.link)
                    .and_then(|f| f.text())
                    .expect("Field link of type text not found")
                    .to_string(),
                content: doc.get_first(self.fields.content)
                    .and_then(|f| f.text())
                    .expect("Field content of type text not found")
                    .to_string(),
                source: doc.get_first(self.fields.source)
                    .and_then(|f| f.text())
                    .expect("Field source of type text not found")
                    .to_string(),
                metadata: HashMap::new(),
            };

            results.push(converted);
        }

        Ok(Box::pin(tokio_stream::iter(results)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempdir::TempDir;
    use tokio_stream::StreamExt;

    use crate::model::Document;
    use crate::search::SearchEngine;
    use crate::search::tantivy_impl::TantivySearchEngine;

    #[tokio::test]
    async fn test_tantivy_search_engine() -> anyhow::Result<()> {
        let index_path = TempDir::new("tantivy_index")?;

        let engine = TantivySearchEngine::new(index_path.path())?;

        let document1 = Document {
            title: "Hello world".to_string(),
            content: "Hello content".to_string(),
            source: "My source".to_string(),
            link: "link1".to_string(),
            metadata: HashMap::new(),
            id: "1".to_string(),
        };

        let document2 = Document {
            title: "Computer science".to_string(),
            content: "Computer science content".to_string(),
            source: "My source".to_string(),
            link: "link2".to_string(),
            metadata: HashMap::new(),
            id: "2".to_string(),
        };

        engine.index(vec![document1, document2.clone()]).await?;

        engine.reader.reload()?;

        let results = engine.search("computer".to_string())?.collect::<Vec<_>>().await;

        assert_eq!(results, vec![document2]);

        Ok(())
    }
}
