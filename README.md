# Crony Researcher

Crony Researcher is tool, to search for similar texts (twins) in large datasets. 

**Scope of Similarity:** The tool strictly analyzes word and string-level similarities, which include typos, missing characters, or changes in word order. It does not evaluate "contextual" or semantic similarities. It operates purely on distance algorithms rather than vector embeddings or neural networks (e.g., siamese networks).

**Note on Performance:** All calculations are performed entirely on the CPU, program will use maximum available CPU cores. There are no plans to support GPU-based calculations.

## Required Data Format

For the program to process the file and work correctly at all, the input file **must be a CSV file**, and the only supported schema for the input data is:

```text
id (number), text (string)
```

**Key Information about the data:**

- The file format must strictly be a **CSV**.
- The **id field is not optional** – each record must have a unique numerical identifier.
- The program will automatically **filter out empty strings** (records with no text will not be taken into account in the search process).

## Arguments (Command-line Options)

The program can be configured during runtime using the following flags:

- `-fz`, `--fuzz-filter` <value>
  **Description:** A value between `0.0` and `1.0`. Used to filter out results that do not achieve a sufficient degree of similarity.
  **Default:** `0.85` - i recommend to keep this value as default.

- `-d`, `--max-distance` <number>
  **Description:** Specifies the maximum allowed distance between two strings for them to be considered similar and included in the final results.
  **Default:** `8`

- `-f`, `--data-path` <path>
  **Description:** Path to the input data file (matching the required schema).
  **Default:** `data.csv`

- `-o`, `--results-path` <path>
  **Description:** Path to the CSV file where the search results will be saved (contains the fields: `query_id`, `twin_id`, `distance`).
  **Default:** `results.csv`

## Example Usage

You will achieve the best performance by compiling and running the program in `release` mode:

```bash
cargo run --release -- --fuzz-filter 0.85 --max-distance 8 --data-path data.csv --results-path results.csv
```

## Benchmark results

To be added soon.

## TODO

- [x] Add unit tests.
- [ ] Add benchmark results.
- [ ] Add more arguments.
