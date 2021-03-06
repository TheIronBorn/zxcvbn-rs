use result::PasswordResult;
use matching::{BaseMatch, MatchData};
use std::collections::HashMap;
use std::cmp;
use regex::Regex;
use chrono::{Local, Datelike};
use keygraph_rs::*;

const BRUTEFORCE_CARDINALITY: u64 = 10;
const MIN_GUESSES_BEFORE_GROWING_SEQUENCE: u64 = 10000;
const MIN_SUBMATCH_GUESSES_SINGLE_CHAR: u64 = 10;
const MIN_SUBMATCH_GUESSES_MULTI_CHAR: u64 = 50;
const MIN_YEAR_SPACE: i32 = 20;

#[derive(Debug)]
struct MatchScores {
    m: BaseMatch,
    pi: u64,
    g: u64,
    length: usize,
}

#[derive(Default, Debug)]
struct OptimalMatch {
    scores: HashMap<usize, Vec<MatchScores>>,
    exclude_additive: bool,
}

impl OptimalMatch {
    fn update(&mut self, pass: &str, m: &BaseMatch, l: usize) {
        let k = m.end;

        let mut pi = estimate_guesses(m, pass);
        if l > 1 {
            assert!(self.scores.contains_key(&(m.start - 1)));
            if let Some(score_list) = self.scores.get(&(m.start - 1)) {
                if let Some(s) = score_list.iter().find(|x| x.length == l - 1) {
                    pi *= s.pi;
                }
            }
        }
        let mut g = factorial(l as u64) * pi;
        if self.exclude_additive {
            g += MIN_GUESSES_BEFORE_GROWING_SEQUENCE.pow((l - 1) as u32);
        }
        if self.scores.contains_key(&k) {
            let scores = self.scores.get_mut(&k).unwrap();
            for scores in scores.iter() {
                if scores.length > l {
                    continue;
                } else if scores.g <= g {
                    return;
                }
            }
        } else {
            self.scores.insert(k, vec![]);
        }

        self.scores.get_mut(&k).unwrap().push(MatchScores {
            m: m.clone(),
            g: g,
            pi: pi,
            length: l,
        });
    }

    fn unwind(&self, n: usize) -> Vec<BaseMatch> {
        let mut result: Vec<BaseMatch> = Vec::new();
        result.reserve(1);
        let mut k = (n as i32) - 1i32;
        let mut l = 0usize;
        let mut g = u64::max_value();
        if let Some(scores) = self.scores.get(&(k as usize)) {
            for score in scores.iter() {
                if score.g < g {
                    g = score.g;
                    l = score.length;
                }
            }
        }
        while k >= 0 {
            if let Some(scores) = self.scores.get(&(k as usize)) {
                if let Some(s) = scores.iter().find(|x| x.length == l) {
                    let ref m = s.m;
                    k = (m.start as i32) - 1i32;
                    result.insert(0, m.clone());
                    l -= 1;
                }
            }
        }
        result
    }
}

fn factorial(n: u64) -> u64 {
    let result = if n < 2 {
        1
    } else {
        (2..(n + 1)).fold(1, |acc, x| acc * x)
    };
    result
}

#[test]
fn factorial_test() {
    assert!(factorial(0) == 1);
    assert!(factorial(1) == 1);
    assert!(factorial(2) == 2);
    assert!(factorial(3) == 6);
    assert!(factorial(10) == 3628800);
}

#[allow(non_snake_case)]
fn nCk(mut n: u64, k: u64) -> u64 {
    let result = if k > n {
        0
    } else if 0 == k {
        1
    } else {
        (1..k + 1).fold(1, |acc, d| {
            n -= 1;
            (acc * (n + 1)) / d
        })
    };
    result
}

#[test]
#[allow(non_snake_case)]
fn nCk_test() {
    assert!(nCk(2, 1) == 2);
    assert!(nCk(2, 2) == 1);
    assert!(nCk(2, 3) == 0);
    assert!(nCk(85, 5) == 32801517);
}


fn bruteforce_match(password: &String, start: usize, end: usize) -> BaseMatch {
    BaseMatch {
        pattern: String::from("Bruteforce"),
        start: start,
        end: end,
        token: password[start..end+1].to_string(),
        data: MatchData::Plain,
    }
}

pub fn most_guessable_match_sequence(password: String,
                                     matches: Vec<BaseMatch>,
                                     exclude_additive: bool)
                                     -> PasswordResult {

    let pref = password.as_str();
    let mut optimal = OptimalMatch {
        exclude_additive: exclude_additive,
        ..Default::default()
    };
    let chars = 0..password.len();
    let matches_by_end = chars.map(|x| matches.iter().filter(|y| y.end == x).collect::<Vec<_>>())
                              .collect::<Vec<_>>();

    for k in 0..password.len() {
        for m in matches_by_end[k].iter() {
            if m.start > 0 {
                // update
                let lengths = optimal.scores
                                     .get(&(m.start - 1))
                                     .iter()
                                     .flat_map(|x| x.into_iter())
                                     .map(|x| x.length)
                                     .collect::<Vec<usize>>();
                for l in lengths.iter() {
                    optimal.update(pref, m, l + 1);
                }
            } else {
                optimal.update(pref, m, 1);
            }
        }
        // Bruteforce update
        let bm = bruteforce_match(&password, 0, k);
        optimal.update(pref, &bm, 1);
        for i in 1..k+1 {
            let bm = bruteforce_match(&password, i, k);

            let lengths = optimal.scores
                                 .get(&(i - 1))
                                 .iter()
                                 .flat_map(|x| x.into_iter())
                                 .map(|x| (x.length, x.m.pattern.clone()))
                                 .collect::<Vec<(usize, String)>>();

            for l in lengths.iter() {
                if l.1 == "Bruteforce".to_string() {
                    continue;
                }
                optimal.update(pref, &bm, l.0 + 1);
            }
        }
    }
    let optimal_seq = optimal.unwind(password.len());
    let optimal_length = optimal_seq.iter().count() - 1;

    // format result based on length
    let guesses = if password.len() == 0 {
        1u64
    } else {
        let mut gs = 1u64;
        if let Some(s) = optimal.scores.get(&(password.len() - 1)) {
            let ms = s.get(optimal_length);
            if ms.is_some() {
                gs = ms.unwrap().g;
            }
        } 
        gs
    };
    let g_log10 = (guesses as f64).log10();

    PasswordResult {
        password: password.clone(),
        guesses: guesses,
        guesses_log10: g_log10,
        sequence: optimal_seq,
        ..Default::default()
    }
}


fn estimate_guesses(m: &BaseMatch, password: &str) -> u64 {
    // Here in coffeescript they dynamically add more struct fields to the
    // match which exist in the result anyway. It just seems so wasteful.
    // gonna think of something better but until then this will suffice.

    let min_guesses = if m.token.len() < password.len() {
        if m.token.len() == 1 {
            MIN_SUBMATCH_GUESSES_SINGLE_CHAR
        } else {
            MIN_SUBMATCH_GUESSES_MULTI_CHAR
        }
    } else {
        1u64
    };
    let guesses = match m.pattern.as_str() {
        "Bruteforce" => bruteforce_guesses(&m),
        "Dictionary" => dictionary_guesses(&m),
        "Repeat" => repeat_guesses(&m),
        "Sequence" => sequence_guesses(&m),
        "Regex" => regex_guesses(&m),
        "Date" => date_guesses(&m),
        "Spatial" => spatial_guesses(&m),
        _ => 0u64,
    };

    cmp::max(guesses, min_guesses)
}


fn bruteforce_guesses(m: &BaseMatch) -> u64 {
    let min_guesses = if m.token.len() == 1 {
        MIN_SUBMATCH_GUESSES_SINGLE_CHAR + 1u64
    } else {
        MIN_SUBMATCH_GUESSES_MULTI_CHAR + 1u64
    };
    cmp::max(min_guesses,
             BRUTEFORCE_CARDINALITY.pow(m.token.len() as u32))
}

fn dictionary_guesses(m: &BaseMatch) -> u64 {
    match m.data {
        MatchData::Dictionary {rank, reversed, ..} => {
            let urank = uppercase_variations(m);
            let l33t_rank = l33t_variations(m);
            let reversed_rank = if reversed {
                2u64
            } else {
                1u64
            };
            (rank as u64) * urank * l33t_rank * reversed_rank
        }
        _ => 0u64,
    }

}

fn uppercase_variations(m: &BaseMatch) -> u64 {
    let token = m.token.as_str();

    if token.to_lowercase() == token {
        return 1u64;
    }
    let first_upper = Regex::new(r"^[A-Z][^A-Z]+$").unwrap();
    let last_upper = Regex::new(r"^[^A-Z]+[A-Z]$").unwrap();
    if token.to_uppercase() == token || first_upper.is_match(token) || last_upper.is_match(token) {
        return 2u64;
    }

    let ucount = token.chars().filter(|x| x.is_uppercase()).count() as u64;
    let lcount = token.chars().filter(|x| x.is_lowercase()).count() as u64;
    let mut variations = 0u64;

    for i in 1..(cmp::min(ucount, lcount) + 1) {
        variations += nCk(ucount + lcount, i);
    }
    variations
}

fn l33t_variations(m: &BaseMatch) -> u64 {
    let mut result = 1u64;
    
    let lower_token = m.token.to_lowercase();
    if let MatchData::Dictionary { ref l33t, .. } = m.data {
        if let Some(ref data) = *l33t {
            
            for (k, v) in data.l33t_subs.iter() {
                let subbed = lower_token.chars()
                                        .filter(|x| x == k)
                                        .count();
                
                let unsubbed = lower_token.chars()
                                          .filter(|x| v.contains(*x))
                                          .count();

                if unsubbed == 0 || subbed == 0 {
                    result *= 2;
                } else {
                    let mut possibilities:u64 = 0;
                    for i in 1..cmp::min(subbed, unsubbed)+1 {
                        possibilities += nCk((subbed+unsubbed) as u64, i as u64);
                    }
                    // Modifier for mutli-sub matcher I implemented that differs
                    // from dropboxes. Might be doing possibilties for each
                    // combo is more accurate.
                    result *= v.chars().count() as u64 * possibilities;
                }
            }
        }
    }
    
    result
}


fn repeat_guesses(m: &BaseMatch) -> u64 {
    if let MatchData::Repeat{ ref base_guesses, ref repeat_count, ..} = m.data {
        *base_guesses * *repeat_count as u64
    } else {
        1u64
    }
}

fn sequence_guesses(m: &BaseMatch) -> u64 {
    assert_eq!(m.pattern, "Sequence");

    let digits = Regex::new(r"\d").unwrap();
    let extremes = Regex::new(r"[019azAZ]").unwrap();
    
    let mut base_guesses = if m.token.len() == 0 {
        0u64
    } else if extremes.is_match(&m.token[0..1]) {
        4u64
    } else if digits.is_match(&m.token[0..1]) {
        10u64
    } else {
        26u64
    };

    match m.data {
        MatchData::Sequence{ref ascending, ..} if !ascending => base_guesses *= 2u64,
        _ => {}
    }
    base_guesses * m.token.chars().count() as u64
}

fn regex_guesses(m: &BaseMatch) -> u64 {
    let class_bases:HashMap<String, u64> = {
        let mut m = HashMap::new();
        m.insert(String::from("alpha lower"), 26);
        m.insert(String::from("alpha upper"), 26);
        m.insert(String::from("alpha"), 52);
        m.insert(String::from("alphanumeric"), 62);
        m.insert(String::from("digits"), 10);
        m.insert(String::from("symbols"), 33);
        m
    };
    let reg_name = match m.data {
        MatchData::Regex{ref name} => name,
        _ => "",
    };
    
    let mut guesses = 1u64;

    if let Some(base) = class_bases.get(reg_name) {
        guesses = base.pow(m.token.chars().count() as u32);  
    } else if reg_name == "recent year" {
        let reference_year = Local::now().year() as i32;
        let year:i32 = m.token.parse().expect("Year not a number");
        let year_space = (year - reference_year).abs();
        guesses = cmp::max(year_space, MIN_YEAR_SPACE) as u64;
    }

    guesses
}

fn date_guesses(m: &BaseMatch) -> u64 {
    let mut result:u64 = 0;
    let reference_year = Local::now().year();
    
    if let MatchData::Date { separator, date} = m.data {
        result = 365u64 * cmp::max(date.year() - reference_year, 
                                  MIN_YEAR_SPACE) as u64;
        if separator != '\0' {
            result *= 4;
        }
    }
    result
}


fn calculate_average_key_degree(keyboard: &Keyboard) -> f64 {
    (keyboard.edge_count() as f64) / (keyboard.node_count() as f64)
}

fn spatial_guesses(m: &BaseMatch) -> u64 {
    let mut guesses:u64 = 0;
    let token_length = m.token.chars().count();
    
    if let MatchData::Spatial{ref graph, ref turns, ref shifted_count} = m.data {
        let (numkeys, average_degree) = if ["qwerty", "dvorak"].contains(&graph.as_ref()) {
            (QWERTY_US.node_count(), 
             calculate_average_key_degree(&*QWERTY_US).round() as u64)
        } else {
            (STANDARD_NUMPAD.node_count(), 
             calculate_average_key_degree(&*STANDARD_NUMPAD).round() as u64)
        };
        for i in 2..token_length {
            let possible_turns = cmp::min(*turns, (i-1)) + 1;
            for j in 1..possible_turns {
                guesses += nCk((i - j) as u64, (j - 1) as u64) * 
                    numkeys as u64 * average_degree.pow(j as u32);
            }
        }
        let shifted_count = *shifted_count;
        let unshifted_count = token_length - shifted_count;
        if unshifted_count == 0 || shifted_count == 0 {
            guesses *= 2;
        } else {
            let temp = cmp::min(unshifted_count, shifted_count)+1;
            let shifted_variations = (1..temp)
                .fold(0, |acc, i| acc + 
                      nCk((shifted_count+unshifted_count) as u64, i as u64));
            guesses *= shifted_variations;                               
        }
    }
    guesses
}

