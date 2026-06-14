# Mappings

## Field Definitions

Mappings define how each field in a document is stored and indexed. They are specified when creating an index.

```json
{
  "mappings": {
    "properties": {
      "title": {
        "type": "text",
        "analyzer": "standard",
        "index": true,
        "store": true
      },
      "price": {
        "type": "float",
        "index": true
      },
      "tags": {
        "type": "keyword",
        "index": true
      }
    }
  }
}
```

## Dynamic Mappings

When a field is not explicitly mapped, Peli Search infers its type from the JSON value:

| JSON Value | Inferred Type |
|------------|---------------|
| string | text |
| number (integer) | integer |
| number (float) | float |
| boolean | boolean |

Dynamic mappings can be disabled:

```json
{
  "mappings": {
    "dynamic": false,
    "properties": {
      "title": { "type": "text" }
    }
  }
}
```

## Supported Types

| Type | Description | Indexed | DocValues |
|------|-------------|---------|-----------|
| `text` | Full-text searchable string | yes | no |
| `keyword` | Exact-match string | yes | yes |
| `integer` | 32-bit integer | yes | yes |
| `long` | 64-bit integer | yes | yes |
| `float` | 32-bit float | yes | yes |
| `double` | 64-bit float | yes | yes |
| `boolean` | true/false | yes | yes |
| `array` | List of values | yes | yes |

## Field Options

| Option | Applies To | Default | Description |
|--------|-----------|---------|-------------|
| `analyzer` | text | standard | Tokenizer/analyzer to use |
| `index` | all | true | Whether to index this field |
| `store` | all | false | Whether to store original value |
| `doc_values` | all | true | Enable column-oriented storage |
