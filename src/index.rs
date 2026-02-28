use dashmap::DashMap;
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHasher;
use std::hash::Hasher;

use crate::distance::{DistanceBuffers, levenshtein_distance_raw, normalize};

#[derive(Debug)]
pub struct SearchResult {
    pub id: usize,
    pub distance: usize,
}

#[derive(Debug)]
pub struct PreparedText {
    pub original: String,
    pub normalized_vec: Vec<char>,
    pub normalized_len: usize,
    pub normalized_hash: u64,
    pub trigrams: Vec<[char; 3]>,
}

/// Only used during building phase, clone will be never used here, and is unneccessary.
#[derive(Debug)]
pub struct IndexBuilder {
    index: DashMap<[char; 3], Vec<usize>>,
    storage: DashMap<usize, PreparedText>,
    min_trigram_match_ratio: f64,
}

/// Main "index" of program, used for searching trigrams. Avoid "clone" at all cost.
#[derive(Debug)]
pub struct Indexer {
    index: FxHashMap<[char; 3], Vec<usize>>,
    storage: FxHashMap<usize, PreparedText>, // make index "freeze" and immutable after building to avoid locks
    min_trigram_match_ratio: f64,
}

impl IndexBuilder {
    pub fn new(match_ratio: f64) -> Self {
        Self {
            index: DashMap::new(),
            storage: DashMap::new(),
            min_trigram_match_ratio: match_ratio.clamp(0.0, 1.0),
        }
    }

    pub fn bulk_add(&self, records: Vec<(usize, String)>) {
        records.into_par_iter().for_each(|(id, text)| {
            self.add_single(id, text);
        });
    }

    fn add_single(&self, id: usize, text: String) {
        if text.trim().is_empty() {
            return;
        }

        let mut tokens = tokenize(&text);
        tokens.sort_unstable();
        tokens.dedup();

        let mut cleaned = String::new();
        let mut sorted = String::new();
        let mut ranges = Vec::new();
        normalize(&text, &mut cleaned, &mut sorted, &mut ranges);

        let normalized_vec: Vec<char> = sorted.chars().collect();
        let normalized_len = normalized_vec.len();
        let normalized_hash = hash_chars(&normalized_vec);

        let trigrams = tokens.clone();

        self.storage.insert(
            id,
            PreparedText {
                original: text,
                normalized_vec,
                normalized_len,
                normalized_hash,
                trigrams,
            },
        );

        for token in tokens {
            self.index.entry(token).or_insert_with(Vec::new).push(id);
        }
    }

    pub fn build(self) -> Indexer {
        let index: FxHashMap<[char; 3], Vec<usize>> = self.index.into_iter().collect();
        let storage: FxHashMap<usize, PreparedText> = self.storage.into_iter().collect();

        Indexer {
            index,
            storage,
            min_trigram_match_ratio: self.min_trigram_match_ratio,
        }
    }
}

impl Indexer {
    pub fn search_by_id(&self, query_id: usize, max_distance: usize) -> Vec<SearchResult> {
        let query = match self.storage.get(&query_id) {
            Some(q) => q,
            None => return vec![],
        };

        let q_chars = &query.normalized_vec;
        let q_len = query.normalized_len;
        let q_hash = query.normalized_hash;
        let tokens = &query.trigrams;

        if tokens.is_empty() {
            return vec![];
        }

        let mut candidates: FxHashMap<usize, usize> = FxHashMap::default();
        for token in tokens {
            if let Some(ids) = self.index.get(token) {
                for &id in ids {
                    if id > query_id {
                        *candidates.entry(id).or_insert(0) += 1;
                    }
                }
            }
        }

        let min_matches = (tokens.len() as f64 * self.min_trigram_match_ratio).ceil() as usize;
        let min_matches = std::cmp::max(1, min_matches);

        let mut bufs = DistanceBuffers::new();
        let mut results = Vec::new();

        for (id, matches) in candidates {
            if matches >= min_matches {
                if let Some(prepared) = self.storage.get(&id) {

                    // Fast pre-filter: length difference > max_distance - impossible match
                    if q_len.abs_diff(prepared.normalized_len) > max_distance {
                        continue;
                    }
                    
                    // trying to avoid costly calculations 
                    if q_len == prepared.normalized_len && q_hash == prepared.normalized_hash {
                        // avoid hash collision - very rare but possible, so we double check with actual chars
                        if q_chars == &prepared.normalized_vec {
                            results.push(SearchResult { id, distance: 0 });
                            continue;
                        }
                    }

                    let dist = levenshtein_distance_raw(
                        q_chars,
                        &prepared.normalized_vec,
                        max_distance,
                        &mut bufs,
                    );

                    if dist <= max_distance {
                        results.push(SearchResult { id, distance: dist });
                    }
                }
            }
        }

        results.sort_unstable_by_key(|r| r.distance);
        results
    }

    pub fn search(&self, query: &str, max_distance: usize) -> Vec<SearchResult> {
        let mut q_cleaned = String::new();
        let mut q_sorted = String::new();
        let mut q_ranges = Vec::new();

        normalize(query, &mut q_cleaned, &mut q_sorted, &mut q_ranges);

        let q_chars: Vec<char> = q_sorted.chars().collect();
        let q_len = q_chars.len();
        let q_hash = hash_chars(&q_chars);

        let mut tokens = tokenize(query);

        if tokens.is_empty() {
            return vec![];
        }

        tokens.sort_unstable();
        tokens.dedup();

        let mut candidates: FxHashMap<usize, usize> = FxHashMap::default();
        for token in &tokens {
            if let Some(ids) = self.index.get(token) {
                for &id in ids {
                    *candidates.entry(id).or_insert(0) += 1;
                }
            }
        }

        let min_matches = (tokens.len() as f64 * self.min_trigram_match_ratio).ceil() as usize;
        let min_matches = std::cmp::max(1, min_matches);

        let mut bufs = DistanceBuffers::new();
        let mut results = Vec::new();

        for (id, matches) in candidates {
            if matches >= min_matches {
                if let Some(prepared) = self.storage.get(&id) {
                    if q_len.abs_diff(prepared.normalized_len) > max_distance {
                        continue;
                    }

                    if q_len == prepared.normalized_len && q_hash == prepared.normalized_hash {
                        if q_chars.as_slice() == prepared.normalized_vec.as_slice() {
                            results.push(SearchResult { id, distance: 0 });
                            continue;
                        }
                    }

                    let dist = levenshtein_distance_raw(
                        &q_chars,
                        &prepared.normalized_vec,
                        max_distance,
                        &mut bufs,
                    );

                    if dist <= max_distance {
                        results.push(SearchResult { id, distance: dist });
                    }
                }
            }
        }

        results.sort_unstable_by_key(|r| r.distance);
        results
    }
}

pub fn tokenize(text: &str) -> Vec<[char; 3]> {
    let mut trigrams = Vec::new();
    let mut window = ['\0'; 3];
    let mut current_word_len = 0;
    let mut chars_processed = 0;

    for c in text.chars().flat_map(|c| c.to_lowercase()) {
        if chars_processed >= 10_000 {
            break;
        }
        chars_processed += 1;

        if c.is_alphanumeric() {
            window[0] = window[1];
            window[1] = window[2];
            window[2] = c;
            current_word_len += 1;

            if current_word_len >= 3 {
                trigrams.push([window[0], window[1], window[2]]);
            }
        } else {
            current_word_len = 0;
        }
    }

    trigrams
}

fn hash_chars(chars: &[char]) -> u64 {
    let mut hasher = FxHasher::default();
    for &c in chars {
        hasher.write_u32(c as u32);
    }
    hasher.finish()
}




// tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_trigrams() {
        let text = "hello world";
        let trigrams = tokenize(text);

        assert_eq!(
            trigrams,
            vec![
                ['h', 'e', 'l'],
                ['e', 'l', 'l'],
                ['l', 'l', 'o'],
                ['w', 'o', 'r'],
                ['o', 'r', 'l'],
                ['r', 'l', 'd']
            ]
        );
    }

    #[test]
    fn test_tokenize_short_words() {
        let text = "a bc def";
        let trigrams = tokenize(text);

        assert_eq!(trigrams, vec![['d', 'e', 'f']]);
    }

    #[test]
    fn test_index_builder_and_search() {
        let builder = IndexBuilder::new(0.5); // 50% 

        builder.bulk_add(vec![
            (1, "hello world".to_string()),
            (2, "hello kitty".to_string()),
            (3, "something else completely".to_string()),
        ]);

        let indexer = builder.build();

        let results = indexer.search("hello", 6);

        assert_eq!(results.len(), 2);

        let mut ids: Vec<usize> = results.iter().map(|r| r.id).collect();

        ids.sort_unstable();

        assert_eq!(ids, vec![1, 2]);

        for result in results {
            assert_eq!(result.distance, 6);
        }
    }

    #[test]
    fn test_search_by_id() {
        let builder = IndexBuilder::new(0.5); // 50% 

        builder.bulk_add(vec![
            (1, "the quick brown fox".to_string()),
            (2, "the fast brown fox".to_string()),
            (3, "unrelated text here".to_string()),
        ]);

        let indexer = builder.build();

        let results = indexer.search_by_id(1, 10);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 2);
    }
}
