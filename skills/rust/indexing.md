# Rust Indexing Examples

```rust
use pelisearch::{AddDocumentRequest, Client};
use std::collections::HashMap;

let client = Client::from_url("http://localhost:7700")?;

// Create index
client.create_index("articles").await?;

// Add single document
let fields = HashMap::from([
    ("title".into(), "Getting Started with Rust".into()),
    ("content".into(), "Rust is a systems programming language...".into()),
]);
client.add_document("articles", "doc-1", fields).await?;

// Bulk add documents
client.bulk_add_documents("articles", vec![
    AddDocumentRequest {
        id: "1".into(),
        fields: HashMap::from([("title".into(), "Rust Ownership".into())]),
    },
    AddDocumentRequest {
        id: "2".into(),
        fields: HashMap::from([("title".into(), "Rust Borrowing".into())]),
    },
]).await?;

// Get / delete document
let doc = client.get_document("articles", "doc-1").await?;
println!("{:?}", doc.get("title"));
client.delete_document("articles", "doc-1").await?;
```
