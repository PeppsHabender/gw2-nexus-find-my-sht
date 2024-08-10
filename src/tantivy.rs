use crate::utils::sub_path;
use std::fs::{create_dir_all, remove_file};
use std::sync::OnceLock;
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT,
};
use tantivy::tokenizer::NgramTokenizer;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Searcher, TantivyDocument};

static mut INDEX: OnceLock<Index> = OnceLock::new();
static mut READER: OnceLock<IndexReader> = OnceLock::new();
static mut WRITER: OnceLock<IndexWriter> = OnceLock::new();

pub fn tantivy_index() -> &'static Index {
    unsafe {
        if let Some(index) = INDEX.get() {
            return index;
        }

        let index_dir = sub_path("index");
        if !index_dir.clone().exists() {
            create_dir_all(index_dir.clone()).expect("dir to be created");
        }

        let index = match Index::create_in_dir(index_dir.clone(), schema()) {
            Ok(index) => index,
            Err(_) => Index::open_in_dir(index_dir.clone()).unwrap(),
        };

        index
            .tokenizers()
            .register("ngram", NgramTokenizer::new(3, 10, false).unwrap());

        let _ = INDEX.set(index);
        tantivy_index()
    }
}

pub fn cleanup_tantivy() {
    unsafe {
        if let Some(writer) = WRITER.take() {
            let _ = writer.wait_merging_threads();
        }

        let _ = READER.take();
        let _ = INDEX.take();

        let _ = remove_file(sub_path("index").join(".tantivy-writer.lock"));
        let _ = remove_file(sub_path("index").join(".tantivy-meta.lock"));
    }
}

pub fn add_documents<T>(iter: T)
where
    T: Iterator<Item = TantivyDocument>,
{
    let writer = writer();
    let _ = writer.delete_all_documents();
    iter.for_each(|d| {
        let _ = writer.add_document(d);
    });

    let _ = writer.prepare_commit();
    let _ = writer.commit();
}

pub fn index_searcher() -> Searcher {
    reader().searcher()
}

fn reader() -> &'static mut IndexReader {
    unsafe {
        if let Some(reader) = READER.get_mut() {
            return reader;
        }

        let r = tantivy_index()
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .unwrap();

        let _ = READER.set(r);
        reader()
    }
}

fn writer() -> &'static mut IndexWriter {
    unsafe {
        if let Some(writer) = WRITER.get_mut() {
            return writer;
        }

        let _ = WRITER.set(tantivy_index().writer(1024 * 1024 * 256).unwrap());
        writer()
    }
}

fn schema() -> Schema {
    let mut schema_builder = Schema::builder();
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("ngram")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    let _ = schema_builder.add_u64_field("id", INDEXED | STORED);
    let _ = schema_builder.add_text_field("name", text_options.clone());
    let _ = schema_builder.add_text_field("description", TEXT);
    let _ = schema_builder.add_bytes_field("item", STORED);

    schema_builder.build()
}

#[derive(Clone)]
pub struct TantivySchema {
    pub id_field: Field,
    pub name_field: Field,
    pub descr_field: Field,
    pub item_field: Field,
}

impl From<Schema> for TantivySchema {
    fn from(value: Schema) -> Self {
        Self {
            id_field: value.get_field("id").unwrap(),
            name_field: value.get_field("name").unwrap(),
            descr_field: value.get_field("description").unwrap(),
            item_field: value.get_field("item").unwrap(),
        }
    }
}
