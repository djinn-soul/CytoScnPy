# CSP-P015: Pandas read_csv Without chunksize

**Category:** `Performance`

**Severity:** `LOW`

## Description

Reading large CSVs without chunksize can exhaust memory. Use chunksize, nrows, or iterator.

## Vulnerable Code Example

```python
df = pandas.read_csv("big.csv")
```

## Safer Code Example

```python
for chunk in pandas.read_csv("big.csv", chunksize=10000):
    handle(chunk)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P015
```
