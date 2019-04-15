use std::cmp::max;

use difference::Difference;

#[allow(dead_code)]
#[derive(Clone, PartialEq)]
pub enum SplitType {
    Character,
    Word,
    Line,
    SmartWord,
}

#[allow(dead_code)]
pub fn diff(orig: &str, edit: &str, split: &SplitType) -> (i32, Vec<Difference>) {
    let ch = Changeset::new(orig, edit, split);
    (ch.distance, ch.diffs)
}

pub struct Changeset {
    pub diffs: Vec<Difference>,
    pub split: SplitType,
    pub distance: i32,
}

impl Changeset {
    pub fn new(orig: &str, edit: &str, split: &SplitType) -> Changeset {
        let (dist, common) = lcs(orig, edit, split);
        Changeset {
            diffs: merge(orig, edit, &common, split),
            split: split.to_owned(),
            distance: dist,
        }
    }
}

fn strsplit<'a>(s: &'a str, split: &str) -> Vec<&'a str> {
    let mut si = s.split(split);
    if split == "" {
        si.next();
    }
    let mut v: Vec<&str> = si.collect();
    if split == "" {
        v.pop();
    }
    v
}

fn smartsplit<'a>(s: &'a str) -> Vec<&'a str> {
    let slice = s.as_bytes();
    let (mut out, last, _, _) = s.chars().fold(
        (Vec::new(), 0, 0, 0),
        |(mut sum, mut last, mut current, mut state), e| {
            let new_state = if e.is_alphanumeric() {
                1
            } else if e == ' ' {
                2
            } else if state < 3 {
                3
            } else {
                state + 1
            } as i32;
            if state != new_state {
                if state > 0 {
                    sum.push(std::str::from_utf8(&slice[last..current]).unwrap());
                }
                last = current;
                current += e.len_utf8();
                state = new_state;
            } else {
                current += e.len_utf8();
            }
            (sum, last, current, state)
        },
    );
    if slice.len() >= last {
        out.push(std::str::from_utf8(&slice[last..]).unwrap());
    }
    //out.push(std::str::from_utf8(&slice[last..]).unwrap());
    eprintln!("u: {:?}", &out);;
    out
}

// finds the longest common subsequences
// outputs the edit distance and a string containing
// all chars both inputs have in common
//
// This algorithm is based on
// https://en.wikipedia.org/wiki/Longest_common_subsequence_problem#Code_for_the_dynamic_programming_solution
#[allow(non_snake_case)]
#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
pub fn lcs(orig: &str, edit: &str, split: &SplitType) -> (i32, String) {
    // make list by custom splits
    let (a, b) = match split {
        SplitType::Character => (strsplit(orig, ""), strsplit(edit, "")),
        SplitType::Word => (strsplit(orig, " "), strsplit(edit, " ")),
        SplitType::Line => (strsplit(orig, "\n"), strsplit(edit, "\n")),
        SplitType::SmartWord => (smartsplit(orig.trim()), smartsplit(edit.trim())),
    };

    let N = a.len();
    let M = b.len();

    let mut idx: Vec<usize> = Vec::with_capacity(N * M);
    idx.resize(N * M, 0);

    for i in 0..N {
        for j in 0..M {
            if b[j] == a[i] {
                if i == 0 || j == 0 {
                    idx[i * M + j] = 1;
                } else {
                    idx[i * M + j] = idx[(i - 1) * M + j - 1] + 1;
                }
            } else if i == 0 {
                if j == 0 {
                    idx[i * M + j] = 0;
                } else {
                    idx[i * M + j] = idx[i * M + j - 1];
                }
            } else if j == 0 {
                idx[i * M + j] = idx[(i - 1) * M + j];
            } else {
                idx[i * M + j] = max(idx[i * M + j - 1], idx[(i - 1) * M + j]);
            }
        }
    }

    let mut i = (N as isize) - 1;
    let mut j = (M as isize) - 1;
    let mut lcs = Vec::new();
    while i >= 0 && j >= 0 {
        let ui = i as usize;
        let uj = j as usize;
        if a[ui] == b[uj] {
            lcs.push(a[ui]);
            i -= 1;
            j -= 1;
        } else if j == 0 && i == 0 {
            break;
        } else if i == 0 || idx[ui * M + uj - 1] > idx[(ui - 1) * M + uj] {
            j -= 1;
        } else {
            i -= 1;
        }
    }

    lcs.reverse();
    (
        (N + M - 2 * lcs.len()) as i32,
        lcs.join(if *split == SplitType::Word { " " } else { "" }),
    )
}

// merges the changes from two strings, given a common substring
pub fn merge(orig: &str, edit: &str, common: &str, split: &SplitType) -> Vec<Difference> {
    let mut ret = Vec::new();

    // make list by custom splits
    let l = match split {
        SplitType::Character => orig.split("").collect(),
        SplitType::Word => orig.split(" ").collect(),
        SplitType::Line => orig.split("\n").collect(),
        SplitType::SmartWord => smartsplit(orig),
    };
    let r = match split {
        SplitType::Character => edit.split("").collect(),
        SplitType::Word => edit.split(" ").collect(),
        SplitType::Line => edit.split("\n").collect(),
        SplitType::SmartWord => smartsplit(edit),
    };
    let c = match split {
        SplitType::Character => common.split("").collect(),
        SplitType::Word => common.split(" ").collect(),
        SplitType::Line => common.split("\n").collect(),
        SplitType::SmartWord => smartsplit(common),
    };
    let mut l = l.iter().map(|v| *v).peekable();
    let mut r = r.iter().map(|v| *v).peekable();
    let mut c = c.iter().map(|v| *v).peekable();

    // Turn empty strings into [], not [""]
    if orig == "" {
        l.next();
    }
    if edit == "" {
        r.next();
    }
    if common == "" {
        c.next();
    }

    //eprintln!("ls");
    while l.peek().is_some() || r.peek().is_some() {

        if let Some(string) = l.peek() {
            eprintln!(r#"lsome "{}", orig: "{}""#, &string, &orig);
        }
        if let Some(string) = c.peek() {
            eprintln!(r#"csome "{}", common: "{}""#, &string, &common);
        }
        if let Some(string) = r.peek() {
            eprintln!(r#"rsome "{}""#, &string);
        }

        eprintln!("l1");
        let mut same = Vec::new();
        while l.peek().is_some() && l.peek() == c.peek() && r.peek() == c.peek() {
        eprintln!("l1.1");
            same.push(l.next().unwrap());
            r.next();
            c.next();
        }
        eprintln!("l2");
        if !same.is_empty() {
        eprintln!("l2.1");
            let joined = same.join(if *split == SplitType::Word { " " } else { "" });
            if joined != "" {
                ret.push(Difference::Same(joined));
            }
        }
        eprintln!("l3");

        let mut rem = Vec::new();
        while l.peek().is_some() && l.peek() != c.peek() {
        eprintln!("l3.1");
            rem.push(l.next().unwrap());
        }
        if !rem.is_empty() {
        eprintln!("l3.2");
            ret.push(Difference::Rem(rem.join(if *split == SplitType::Word {
                " "
            } else {
                ""
            })));
        }
        eprintln!("l4");

        let mut add = Vec::new();
        while r.peek().is_some() && r.peek() != c.peek() {
        eprintln!("l4.1");
            add.push(r.next().unwrap());
        }
        if !add.is_empty() {
        eprintln!("l4.2");
            ret.push(Difference::Add(add.join(if *split == SplitType::Word {
                " "
            } else {
                ""
            })));
        }
        eprintln!("l5");
    }
    eprintln!("le");

    ret
}
