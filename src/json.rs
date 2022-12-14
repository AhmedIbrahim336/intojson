use regex::Regex;
use serde_json::Value;

use crate::utils::{get_type, should_skip, ValueType, get_string, to_json_obj};
use std::fmt::Display;
use std::{fs, io::Error, path::Path, process};

#[derive(Debug)]
pub struct Json {
    path: String,
    blocks: Vec<Block>,
}

impl Json {
    pub fn is_block(line: &str) -> bool {
        !should_skip(line) && line.trim().starts_with("[")
    }

    pub fn from_file<P: AsRef<Path> + Display>(path: P) -> Result<Json, Error> {
        let toml_str = fs::read_to_string(&path)?;
        let lines = toml_str
            .split("\n")
            .into_iter()
            .map(|l| l.trim())
            .collect::<Vec<&str>>();

        let mut idx = 0;
        let mut blocks = vec![];

        while idx < lines.len() {
            let line = lines[idx];
            if should_skip(line) {
                idx += 1;
                continue;
            }

            if Json::is_block(line) {
                blocks.push(Json::parse_block(&lines, idx));
            }

            idx += 1;
        }

        Ok(Json {
            path: path.to_string(),
            blocks,
        })
    }

    pub fn parse_block(lines: &Vec<&str>, idx: usize) -> Block {
        let line = lines[idx];

        if !Json::is_block(line) {
            eprint!("Invalid TOML block");
            process::exit(0);
        }

        let re = Regex::new(r"\[(?P<name>[^\]]+)\]").expect("Invalid regex");
        let block_name = match re.captures(line) {
            Some(cap) => cap["name"].to_string(),
            None => {
                eprintln!("Invalid TOML");
                process::exit(0);
            }
        };

        let mut end_idx = idx;
        loop {
            end_idx += 1;

            if end_idx >= lines.len() {
                break;
            }

            let line = lines[end_idx];
            if Json::is_block(line) || should_skip(line) {
                break;
            }
        }

        let block_lines = &lines[idx + 1..end_idx];
        Block::new(&block_name, block_lines)
    }

    fn to_json_value(&self) -> Result<Value, String> {
        let json_str = self
            .blocks
            .iter()
            .map(|block| block.to_json())
            .collect::<Vec<String>>()
            .join(",");
        let json_str = format!(r"{{{}}}", json_str);
        serde_json::from_str(&json_str).map_err(|_| "Invalid json structure".into())
    }

    pub fn save(&self) -> Result<(), Error> {
        let output = self.path.replace(".toml", ".json");
        let json = self.to_json_value().unwrap();
        let json = serde_json::to_string_pretty(&json).unwrap();
        fs::write(output, json)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block {
    pub name: String,
    pub entries: Vec<Entry>,
}

impl Block {
    fn new(name: &str, lines: &[&str]) -> Self {
        let entries = lines.into_iter().map(|&line| Entry::new(line)).collect();

        Self {
            name: name.to_string(),
            entries,
        }
    }

    pub fn to_json(&self) -> String {
        let key = self.name.clone();
        let value = self
            .entries
            .iter()
            .map(|attr| attr.to_raw_json())
            .collect::<Vec<String>>()
            .join(",");
        let value: Value = serde_json::from_str(&format!("{{{}}}", value)).unwrap();
        format!(r#""{}": {}"#, key, value.to_string())
    }
}

#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub value: String,
}

impl Entry {
    pub fn new(line: &str) -> Self {
        let re = Regex::new(r#"(?P<key>[^=\n]+)=\s*(?P<value>.*)"#).unwrap();
        let (key, value) = match re.captures(line) {
            Some(cap) => (
                cap["key"].trim().to_string(),
                cap["value"].trim().to_string(),
            ),
            None => {
                eprintln!(r#""{line}" is not a valid TOML"#);
                process::exit(0);
            }
        };

        Self { key, value }
    }

    pub fn to_raw_json(&self) -> String {
        let value_type = get_type(&self.value);
        // println!("{:#?} -=> {:#?}", self.value, value_type);

        let value = match value_type {
            ValueType::String => format!(r#""{}""#, get_string(&self.value)),
            ValueType::Object => to_json_obj(&self.value),
            _ => self.value.to_string(),
        };

        format!(r#""{}": {}"#, self.key, value)
    }
}
