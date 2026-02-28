use std::cmp::min;

/// This struct is used to store the intermediate results of the distance calculations.
/// It is used to avoid reallocating strings and vectors all the time.
#[derive(Clone)]
pub struct DistanceBuffers {
    pub cleaned_a: String,
    pub sorted_a: String,
    pub cleaned_b: String,
    pub sorted_b: String,
    pub ranges: Vec<(usize, usize)>,
    pub char_a: Vec<char>,
    pub char_b: Vec<char>,
    pub cache: Vec<usize>,
}

impl Default for DistanceBuffers {
    fn default() -> Self {
        Self::new()
    }
}

impl DistanceBuffers {
    pub fn new() -> Self {
        Self {
            // This is to ensure that we dont have to reallocate strings and vectors all the time, sizes are chosen by me "eye"
            cleaned_a: String::with_capacity(256),
            cleaned_b: String::with_capacity(256),

            sorted_a: String::with_capacity(256),
            sorted_b: String::with_capacity(256),

            char_a: Vec::with_capacity(256),
            char_b: Vec::with_capacity(256),

            cache: Vec::with_capacity(256),
            ranges: Vec::with_capacity(32),
        }
    }
}

// pub fn token_sort_ratio(
//     a: &str,
//     b: &str,
//     max_distance: usize,
//     bufs: &mut DistanceBuffers,
// ) -> usize {
//     normalize(a, &mut bufs.cleaned_a, &mut bufs.sorted_a, &mut bufs.ranges);
//     normalize(b, &mut bufs.cleaned_b, &mut bufs.sorted_b, &mut bufs.ranges);

//     bufs.char_a.clear();
//     bufs.char_a.extend(bufs.sorted_a.chars());

//     bufs.char_b.clear();
//     bufs.char_b.extend(bufs.sorted_b.chars());
//     levenshtein_distance(max_distance, bufs)
// }

pub fn normalize(
    s: &str,
    cleaned_buf: &mut String,
    sorted_buf: &mut String,
    token_ranges: &mut Vec<(usize, usize)>,
) {
    cleaned_buf.clear();

    for c in s.chars() {
        if c.is_alphanumeric() || c.is_whitespace() {
            for lc in c.to_lowercase() {
                cleaned_buf.push(lc);
            }
        }
    }

    token_ranges.clear();
    let mut start = 0;
    let mut in_word = false;

    for (i, c) in cleaned_buf.char_indices() {
        if c.is_whitespace() {
            if in_word {
                token_ranges.push((start, i));
                in_word = false;
            }
        } else if !in_word {
            start = i;
            in_word = true;
        }
    }
    if in_word {
        token_ranges.push((start, cleaned_buf.len()));
    }

    token_ranges.sort_unstable_by_key(|&(s, e)| &cleaned_buf[s..e]);

    sorted_buf.clear();
    for (i, &(s, e)) in token_ranges.iter().enumerate() {
        if i > 0 {
            sorted_buf.push(' ');
        }
        sorted_buf.push_str(&cleaned_buf[s..e]);
    }
}

// fn levenshtein_distance(max_distance: usize, bufs: &mut DistanceBuffers) -> usize {
//     let a_len = bufs.char_a.len();
//     let b_len = bufs.char_b.len();

//     if a_len.abs_diff(b_len) > max_distance {
//         return max_distance + 1;
//     }

//     if a_len == 0 {
//         return b_len;
//     }
//     if b_len == 0 {
//         return a_len;
//     }

//     let (target, source) = if a_len > b_len {
//         (&bufs.char_b[..], &bufs.char_a[..])
//     } else {
//         (&bufs.char_a[..], &bufs.char_b[..])
//     };

//     let m = target.len();
//     let max_val = max_distance + 1;

//     bufs.cache.clear();
//     bufs.cache.extend((0..=m).map(|x| min(x, max_val)));

//     for (i, &s_char) in source.iter().enumerate() {
//         let row = i + 1;
//         let start = if row > max_distance {
//             row - max_distance
//         } else {
//             1
//         };
//         let end = min(m, row + max_distance);

//         let mut diagonal = bufs.cache[start - 1];

//         if start == 1 {
//             bufs.cache[0] = row;
//         } else {
//             bufs.cache[start - 1] = max_val;
//         }

//         let mut min_in_row = max_val;

//         for j in (start - 1)..end {
//             let t_char = target[j];
//             let next_diagonal = bufs.cache[j + 1];

//             let cost = if s_char == t_char { 0 } else { 1 };

//             bufs.cache[j + 1] = min(
//                 min(bufs.cache[j + 1] + 1, bufs.cache[j] + 1),
//                 diagonal + cost,
//             );

//             min_in_row = min(min_in_row, bufs.cache[j + 1]);
//             diagonal = next_diagonal;
//         }

//         if min_in_row > max_distance {
//             return max_val;
//         }
//     }

//     if bufs.cache[m] <= max_distance {
//         bufs.cache[m]
//     } else {
//         max_val
//     }
// }

/// This is implementation of bounded Levenshtein - ukkonen's algorithm
/// https://en.wikipedia.org/wiki/Levenshtein_distance ; https://en.wikipedia.org/wiki/Ukkonen%27s_algorithm
pub fn levenshtein_distance_raw(
    a: &[char],
    b: &[char],
    max_distance: usize,
    bufs: &mut DistanceBuffers,
) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len.abs_diff(b_len) > max_distance {
        return max_distance + 1;
    }

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let (target, source) = if a_len > b_len { (b, a) } else { (a, b) };

    let m = target.len();
    let max_val = max_distance + 1;

    // Re-use the pre-allocated cache buffer
    bufs.cache.clear();
    bufs.cache.extend((0..=m).map(|x| min(x, max_val)));

    // assert!(bufs.cache.len() > m); // might be usefull for compiler

    for (i, &s_char) in source.iter().enumerate() {
        let row = i + 1;
        let start = if row > max_distance {
            row - max_distance
        } else {
            1
        };
        let end = min(m, row + max_distance);

        let mut diagonal = bufs.cache[start - 1];
        if start == 1 {
            bufs.cache[0] = row;
        } else {
            bufs.cache[start - 1] = max_val;
        }

        let mut min_in_row = max_val;

        for j in (start - 1)..end {
            let t_char = target[j];
            let next_diagonal = bufs.cache[j + 1];
            let cost = if s_char == t_char { 0 } else { 1 };

            let res = min(
                min(bufs.cache[j + 1] + 1, bufs.cache[j] + 1),
                diagonal + cost,
            );

            bufs.cache[j + 1] = res;
            if res < min_in_row {
                min_in_row = res;
            }
            diagonal = next_diagonal;
        }

        if min_in_row > max_distance {
            return max_val;
        }
    }

    if bufs.cache[m] <= max_distance {
        bufs.cache[m]
    } else {
        max_val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let mut cleaned_buf = String::new();
        let mut sorted_buf = String::new();
        let mut token_ranges = Vec::new();

        normalize(
            "Hello World!",
            &mut cleaned_buf,
            &mut sorted_buf,
            &mut token_ranges,
        );

        assert_eq!(cleaned_buf, "hello world");
        assert_eq!(sorted_buf, "hello world");
        assert_eq!(token_ranges, vec![(0, 5), (6, 11)]);

        normalize(
            "World Hello",
            &mut cleaned_buf,
            &mut sorted_buf,
            &mut token_ranges,
        );
        assert_eq!(sorted_buf, "hello world");

        normalize(
            "  Test   cases!!!  ",
            &mut cleaned_buf,
            &mut sorted_buf,
            &mut token_ranges,
        );
        assert_eq!(cleaned_buf, "  test   cases  ");
        assert_eq!(sorted_buf, "cases test");
    }

    #[test]
    fn test_levenshtein_distance_raw() {
        let mut bufs = DistanceBuffers::new();

        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        let max_distance = 10;

        let dist = levenshtein_distance_raw(&a, &b, max_distance, &mut bufs);
        assert_eq!(dist, 3);

        let a2: Vec<char> = "flaw".chars().collect();
        let b2: Vec<char> = "lawn".chars().collect();
        let dist2 = levenshtein_distance_raw(&a2, &b2, 10, &mut bufs);
        assert_eq!(dist2, 2);

        let dist_exceeds = levenshtein_distance_raw(&a, &b, 2, &mut bufs);
        assert_eq!(dist_exceeds, 3);

        let dist_exact = levenshtein_distance_raw(&a, &a, 10, &mut bufs);
        assert_eq!(dist_exact, 0);
    }
}
