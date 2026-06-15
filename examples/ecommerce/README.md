# Ecommerce Product Search

Search a product catalog with filters and sorting.

## Prerequisites

Start PeliSearch on `http://localhost:7700`, then run one of the examples below.

## JavaScript

```bash
cd examples/ecommerce/javascript
npm install
node main.mjs
```

## Python

```bash
cd examples/ecommerce/python
pip install -e ../../../sdk/python
python main.py
```

## What It Does

1. Creates a `products` index
2. Indexes sample products (title, category, price)
3. Searches for "keyboard" with a category filter
4. Sorts results by price
