#![feature(portable_simd)]
#![feature(drain_filter)]
use std::{
    fs::File,
    io::{prelude::*, BufReader, Write, stdout},
    path::Path,
    collections::HashMap,
    time::Instant
};

use core_simd::*;

struct Word {
    string: String,
    string_sum: i32,
    char_tbl: [i8; 256]
}

struct WordList {
    words: Vec<Word>,
}

struct WordListIter<'a> {
    words: &'a Vec<Word>,
    idx: usize,
    idx_end: usize,
}

impl WordList {
    fn from_file(path: &str) -> Self {
        let mut raw_lines = lines_from_file(path);
        raw_lines.sort_by(|a, b| {
            if a.len() == b.len() {
                Word::_string_sum(a).cmp(&Word::_string_sum(b))
            } else {
                a.len().cmp(&b.len())
            }
        });
        let lines = raw_lines.into_iter().map(|s| Word::new(s)).collect::<Vec<Word>>();

        Self {
            words: lines,
        }
    }

    fn len(&self) -> usize {
        self.words.len()
    }

    fn segments(&self) -> WordListIter {
        WordListIter {
            words: &self.words,
            idx: 0,
            idx_end: 0
        }
    }
}

impl<'a> Iterator for WordListIter<'a> {
    type Item = Vec<Option<&'a Word>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx_end >= self.words.len() {
            return None;
        }
        let seg_word_len = self.words.get(self.idx).unwrap().len();
        self.idx = self.idx_end;
        while self.words.get(self.idx_end).map(|v| v.len()) == Some(seg_word_len) {
            self.idx_end += 1;
        }
        
        Some((self.words[self.idx..self.idx_end]).iter().map(|s| Some(s)).collect())

        
    }
}


impl Word {
    fn _char_tbl(string: &str) -> [i8; 256] {
        let mut table = [0i8; 256];
        for ch in string.as_bytes() {
            table[*ch as usize] += 1;
        }
        table
    }
    
    fn _string_sum(string: &str) -> i32 {
        let mut sum = 0;
        for ch in string.as_bytes() {
            sum += *ch as i32;
        }
        sum
    }

    fn new(string: String) -> Self {
        Self {
            char_tbl: Self::_char_tbl(&string),
            string_sum: Self::_string_sum(&string),
            string,
        }
    }
    
    #[inline(always)]
    fn compare(&self, other: &Word) -> bool {
        if self.string_sum != other.string_sum {
            return false
        }
        let mut anagramatic = true;
        for i in 0..4 {
            anagramatic = anagramatic && 
            (i8x64::from_slice(&self.char_tbl[i*64..i*64+64])
            == i8x64::from_slice(&other.char_tbl[i*64..i*64+64]));
        }

        anagramatic
    }

    fn len(&self) -> usize {
        self.string.len()
    }
}

fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

fn progress(done: u32, total: u32) -> String {
    let p = (done as f64) / (total as f64);
    let dots = (p * 20f64).ceil() as usize;

    let mut out = String::from("");
    for i in 0..20 {
        if i <= dots {
            out.push('#');
        } else {
            out.push(' ');
        }
    }
    out
}

fn main() {
    println!("Reading and preprocessing vocabulary...");
    let vocabulary = WordList::from_file("alpha_words.txt");
    let mut stdout = stdout();
    let mut done = 0;
    let total = vocabulary.len();

    let mut groups: HashMap<&String, Vec<&Word>> = HashMap::new();

    println!("Using vocabulary of {} words.", total);
    println!("Starting search...");
    let begin = Instant::now();
    for mut segment in vocabulary.segments() {
        'outer: loop {

            let word = 'inner: loop {
                match segment.pop() {
                    Some(w) => {match w {
                        Some(s) => {break 'inner s},
                        None => continue 'inner
                    };},
                    None => break 'outer
                }
            };
    
            if done % 10_000 == 0 {
                print!("\rProgress: [{}]", progress(done, total as u32));
                stdout.flush().unwrap();
            }
            done += 1;
            
            let mut anagrams = Vec::new();
            let mut candidates = Vec::new();
            for cndt in segment.iter_mut().rev() {
                if cndt.unwrap().string_sum == word.string_sum {
                    candidates.push(cndt);
                } else {
                    break;
                }
            }

            for candidate in candidates {
                if word.compare(candidate.unwrap()) {
                    anagrams.push(candidate.take().unwrap());
                }
            }

            if !anagrams.is_empty() {

                done += anagrams.len() as u32;
                segment.drain_filter(|w| w.is_none());

                groups.insert(&word.string, anagrams);
            }
        }
    }
    println!();

    let elapsed = begin.elapsed().as_millis();
    println!("Finished finding anagrams in {}ms. Writing to file...", elapsed);

    let mut file_buf = Vec::new();

    for (word, anagrams) in groups.into_iter() {
        let mut buf = String::new();
        buf.push_str(&format!("{}:\n", word));
        for word in anagrams.into_iter() {
            buf.push_str(&format!("   - {}\n", word.string))
        }
        buf.push_str("------------\n");
        file_buf.push(buf);
    }

    let mut anagram_file = File::create("anagrams.txt").unwrap();
    for entry in file_buf.iter() {
        anagram_file.write_all(entry.as_bytes()).unwrap();
    }
    println!("Done! <3");
}
