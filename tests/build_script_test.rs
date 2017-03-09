include!(concat!(env!("OUT_DIR"), "/frequency_data.rs"));
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::io;
use std::io::Read;



#[test]
fn no_duplicates_smart() {
    let dicts = vec![FEMALE_NAMES, MALE_NAMES, SURNAMES, PASSWORDS,
                     ENGLISH_WIKIPEDIA, US_TV_AND_FILM];
    let dict_names = vec!["Female names", "Male names", "Surnames", "Passwords",
                          "Wikipedia", "TV and Film"];

    let mut map:HashSet<String> = HashSet::new();

    for d in dicts.iter() {
        for word in d.to_vec() {
            assert!(!map.contains(&word.to_string()), 
                    "Failed {} appears more than once", word);
            map.insert(word.to_string());
        }
    }
}
