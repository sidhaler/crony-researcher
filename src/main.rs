use crony_researcher::index::IndexBuilder;
use rayon::prelude::*;
use std::error::Error;
use std::fs::File;
use std::time::Instant;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt()]
/// SCAN OF ANY TWINS IN DATASETS
///
/// USAGE: cargo run --release -- --fuzz-filter <fuzz_filter> --max-distance <max_distance> --data-path <data_path> --results-path <results_path>
///
/// EXAMPLE: cargo run --release -- --fuzz-filter 0.85 --max-distance 8 --data-path data.csv --results-path results.csv
struct Opt {
    /// fuzz filter is a value between 0 and 1 that is used to filter out results that are not similar enough
    #[structopt(short = "fz", long = "fuzz-filter", default_value = "0.85")]
    fuzz_filter: f64,
    /// max distance is the maximum distance between two strings that are considered similar
    #[structopt(short = "d", long = "max-distance", default_value = "8")]
    max_distance: usize,
    /// data path is the path to the CSV file that contains the data
    #[structopt(short = "f", long = "data-path", default_value = "data.csv")]
    data_path: String,
    /// results path is the path to the CSV file that will contain the results
    #[structopt(short = "o", long = "results-path", default_value = "results.csv")]
    results_path: String,
}

fn main() {
    // parse arguments
    let opt = Opt::from_args();

    println!(
        "====================================================================================
         \n Welcome in crony researcher i will search for any similarities!
         \n===================================================================================="
    );

    // arguments
    let fuzz_filter = opt.fuzz_filter;
    let max_distance = opt.max_distance;
    let data_path = opt.data_path;
    let results_path = opt.results_path;

    let start = Instant::now();

    println!("Loading data from CSV...");
    let data = match load_data_from_csv(&data_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading CSV: {}", e);
            return;
        }
    };
    println!("Data loaded successfully.\n");

    let query_ids: Vec<usize> = data.iter().map(|(id, _)| *id).collect();

    println!("Indexing {} records...", data.len());

    let builder = IndexBuilder::new(fuzz_filter);

    builder.bulk_add(data);

    let indexer = builder.build();
    println!("Indexing completed\n");

    println!("Starting to search for twins...");
    let search_start = Instant::now();

    let mut saved_results: Vec<SimilarityResult> = query_ids
        .into_par_iter()
        .flat_map_iter(|query_id| {
            indexer
                .search_by_id(query_id, max_distance)
                .into_iter()
                .map(move |a| SimilarityResult {
                    query_id,
                    twin_id: a.id,
                    distance: a.distance,
                })
        })
        .collect();

    // there must be something to replace unstable sort
    saved_results.sort_unstable_by_key(|r| r.query_id);

    let duration_search = search_start.elapsed();

    println!("--------------------------------------------------");
    println!("Time elapsed on search: {:?}", duration_search);
    // println!("Time in microseconds: {}", duration_search.as_micros()); just for debug
    println!("Total unique twins found: {}", saved_results.len());

    let duration = start.elapsed();

    match save_results_to_csv(&saved_results, &results_path) {
        Ok(_) => println!("Results saved to results.csv"),
        Err(e) => eprintln!("Error saving results: {}", e),
    }

    println!("\nProgram execution time: {:?}", duration);
}

#[derive(Debug)]
pub struct SimilarityResult {
    pub query_id: usize,
    pub twin_id: usize,
    pub distance: usize,
}

fn load_data_from_csv(file_path: &str) -> Result<Vec<(usize, String)>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut records = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let id: usize = record[0].parse()?;
        let text: String = record[1].to_string();

        records.push((id, text));
    }

    Ok(records)
}

fn save_results_to_csv(
    results: &Vec<SimilarityResult>,
    file_path: &str,
) -> Result<(), Box<dyn Error>> {
    let file = File::create(file_path)?;
    let mut wtr = csv::Writer::from_writer(file);

    // headers
    wtr.write_record(&["query_id", "twin_id", "distance"])?;

    for result in results {
        wtr.write_record(&[
            result.query_id.to_string(),
            result.twin_id.to_string(),
            result.distance.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
