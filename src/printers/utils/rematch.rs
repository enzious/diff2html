use std::cmp::min;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{ Hash, Hasher, };

pub struct Match {
    index_a: usize,
    index_b: usize,
    _score: f64,
}

pub struct Rematcher<T: Hash + Debug> {
    distance_function: fn(&T, &T) -> f64,
}

impl<T: Hash + Debug> Rematcher<T> {

    pub fn new(distance_function: fn(&T, &T) -> f64) -> Rematcher<T> {
        Rematcher {
            distance_function: distance_function,
        }
    }

    fn find_best_match(&self, a: &[T], b: &[T], cache: &mut HashMap<u64, f64>) -> Option<Match> {
        let mut best_match_dist = ::std::f64::MAX;
        let mut best_match = None;

        let mut hasher = DefaultHasher::new();
        for i in 0..a.len() {
            for j in 0..b.len() {
                ( &a[i], &b[j], ).hash(&mut hasher);
                let cache_key = hasher.finish();
                let md = if cache.contains_key(&cache_key) {
                    cache[&cache_key]
                } else {
                    let distance_function = self.distance_function;
                    cache.insert(cache_key, distance_function(&a[i], &b[j]));
                    cache[&cache_key]
                };
                if md < best_match_dist {
                    best_match_dist = md;
                    best_match = Some(Match {
                        index_a: i,
                        index_b: j,
                        _score: best_match_dist,
                    });
                }
            }
        }

        return best_match;
    }

    fn find_best_match_ref(&self, a: &[&T], b: &[&T], cache: &mut HashMap<u64, f64>) -> Option<Match> {
        let mut best_match_dist = ::std::f64::MAX;
        let mut best_match = None;

        let mut hasher = DefaultHasher::new();
        for i in 0..a.len() {
            for j in 0..b.len() {
                ( &a[i], &b[j], ).hash(&mut hasher);
                let cache_key = hasher.finish();
                let md = if cache.contains_key(&cache_key) {
                    cache[&cache_key]
                } else {
                    let distance_function = self.distance_function;
                    cache.insert(cache_key, distance_function(&a[i], &b[j]));
                    cache[&cache_key]
                };
                if md < best_match_dist {
                    best_match_dist = md;
                    best_match = Some(Match {
                        index_a: i,
                        index_b: j,
                        _score: best_match_dist,
                    });
                }
            }
        }

        return best_match;
    }

    pub fn matches<'a>(&self, a: &'a Vec<T>, b: &'a Vec<T>) -> Vec<Vec<&'a [T]>> {
        self._group(&a[..], &b[..], None, &mut None)
    }

    pub fn matches_ref<'a>(&self, a: &'a Vec<&'a T>, b: &'a Vec<&'a T>) -> Vec<Vec<&'a [&'a T]>> {
        self._group_ref(&a[..], &b[..], None, &mut None)
    }

    fn _group<'a>(&self, a: &'a [T], b: &'a [T], mut level: Option<usize>, cache: &mut Option<HashMap<u64, f64>>) -> Vec<Vec<&'a [T]>> {

        if cache.is_none() {
            *cache = Some(HashMap::new());
        }

        let bm = self.find_best_match(a, b, cache.as_mut().unwrap());

        if level.is_none() {
            level = Some(0);
        }

        let level = level.unwrap();

        if bm.is_none() || (a.len() + b.len() < 3) {
            return vec![vec![a, b]];
        }

        let bm = bm.as_ref().unwrap();

        let a1 = &a[0..bm.index_a];
        let b1 = &b[0..bm.index_b];
        let a_match = &a[bm.index_a..bm.index_a + 1];
        let b_match = &b[bm.index_b..bm.index_b + 1];
        let tail_a = bm.index_a + 1;
        let tail_b = bm.index_b + 1;
        let a2 = &a[tail_a..];
        let b2 = &b[tail_b..];

        let group1 = self._group(a1, b1, Some(level + 1), cache);
        let group_match = self._group(a_match, b_match, Some(level + 1), cache);
        let group2 = self._group(a2, b2, Some(level + 1), cache);
        let mut result = group_match;

        if bm.index_a > 0 || bm.index_b > 0 {
            result = [ group1, result, ].concat();
        }

        if a.len() > tail_a || b.len() > tail_b {
            result = [ result, group2, ].concat();
        }

        result
    }

    fn _group_ref<'a>(&self, a: &'a [&'a T], b: &'a [&'a T], mut level: Option<usize>, cache: &mut Option<HashMap<u64, f64>>) -> Vec<Vec<&'a [&'a T]>> {

        if cache.is_none() {
            *cache = Some(HashMap::new());
        }

        let bm = self.find_best_match_ref(a, b, cache.as_mut().unwrap());

        if level.is_none() {
            level = Some(0);
        }

        let level = level.unwrap();

        if bm.is_none() || (a.len() + b.len() < 3) {
            return vec![vec![a, b]];
        }

        let bm = bm.as_ref().unwrap();

        let a1 = &a[0..bm.index_a];
        let b1 = &b[0..bm.index_b];
        let a_match = &a[bm.index_a..bm.index_a + 1];
        let b_match = &b[bm.index_b..bm.index_b + 1];
        let tail_a = bm.index_a + 1;
        let tail_b = bm.index_b + 1;
        let a2 = &a[tail_a..];
        let b2 = &b[tail_b..];

        let group1 = self._group_ref(a1, b1, Some(level + 1), cache);
        let group_match = self._group_ref(a_match, b_match, Some(level + 1), cache);
        let group2 = self._group_ref(a2, b2, Some(level + 1), cache);
        let mut result = group_match;

        if bm.index_a > 0 || bm.index_b > 0 {
            result = [ group1, result, ].concat();
        }

        if a.len() > tail_a || b.len() > tail_b {
            result = [ result, group2, ].concat();
        }

        result
    }

}

fn levenshtein(a: &str, b: &str) -> usize {

    if a.len() == 0 {
        return b.len();
    }
    if b.len() == 0 {
        return a.len();
    }

    let mut matrix = vec![vec![0usize; a.len() + 1]; b.len() + 1];

    // Increment along the first column of each row
    for i in 0..=b.len() {
        matrix[i][0] = i;
    }

    // Increment each column in the first row
    for j in 0..=a.len() {
        matrix[0][j] = j;
    }

    // Fill in the rest of the matrix
    for i in 1..=b.len() {
        for j in 1..=a.len() {
            if b.chars().nth(i - 1) == a.chars().nth(j - 1) {
                matrix[i][j] = matrix[i - 1][j - 1];
            } else {
                matrix[i][j] = min(
                    matrix[i - 1][j - 1] + 1, // Substitution
                    min(
                        matrix[i][j - 1] + 1, // Insertion
                        matrix[i - 1][j] + 1
                    )
                ); // Deletion
            }
        }
    }

    matrix[b.len()][a.len()]

}

pub fn distance(x: &str, y: &str) -> f64 {
    let x = x.trim();
    let y = y.trim();
    let lev = levenshtein(x, y);
    let _score = lev as f64 / (x.len() as f64 + y.len() as f64);
    _score
}

/*
Copyright (c) 2011 Andrei Mackenzie
Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/