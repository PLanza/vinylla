use crate::img_to_ascii::AsciiArt;

use reqwest::blocking::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Result;
use std::path::Path;

const COLLECTION_PATH: &str = "data/collection.json";

// A struct containing a track's data
#[derive(Serialize, Deserialize, Debug)]
pub struct Track {
    pub(crate) title: String,
    pub(crate) duration: String,
    pub(crate) position: String,
}

// A struct containing a record's data
#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    pub(crate) title: String,
    pub(crate) artists: Vec<String>,
    pub(crate) year: u16,
    pub(crate) genre: Vec<String>,
    pub(crate) style: Vec<String>,
    pub(crate) country: String,
    pub(crate) format: String,
    pub(crate) image: AsciiArt<45, 20>,
    pub(crate) tracklist: Vec<Track>,
}

// A RecordCollection is indexed on a pair of strings containing the first artist of the album, and
// the title of the album
pub type RecordCollection = HashMap<(String, String), Record>;

// Loads the user's collection from the serialized data file into a RecordCollection object
pub fn load_collection() -> Result<RecordCollection> {
    let mut collection: HashMap<(String, String), Record> = HashMap::new();
    if Path::new(COLLECTION_PATH).exists() {
        let data_string = std::fs::read_to_string(COLLECTION_PATH)?;
        let v: Vec<Record> = serde_json::from_str(data_string.as_str())?;
        for record in v {
            collection.insert((record.artists[0].clone(), record.title.clone()), record);
        }
    }

    Ok(collection)
}

impl Record {
    // Returns a record from the json data returned by the Discogs API
    pub fn from_discogs(record_data: Value) -> Result<Record> {
        // Takes the names of the artists data list and adds them to the records artists Vec
        let artists: Vec<String> = record_data["artists"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| process_artist(a["name"].clone()))
            .collect();

        let genre: Vec<String> = match record_data["genres"].as_array() {
            Some(vec) => vec
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            None => Vec::new(),
        };

        let style: Vec<String> = match record_data["styles"].as_array() {
            Some(vec) => vec
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            None => Vec::new(),
        };

        // Takes the first format from the Discogs data, and formats it to a string
        let format = &record_data["formats"].as_array().unwrap()[0];
        let format_str = format!(
            "{}: {}",
            format["name"].as_str().unwrap(),
            format["descriptions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d.as_str().unwrap())
                .collect::<Vec<&str>>()
                .join(", ")
        );

        // Retrieves the album cover image url from the record's json data...
        let img_url = record_data["images"].as_array().unwrap()[0]["resource_url"]
            .as_str()
            .unwrap();
        // ... uses it to send a GET request to retrieve the image bytes
        let img_bytes = get(img_url).unwrap().bytes().unwrap();
        // and loads it as an image to later be converted into AsciiArt
        let image = image::load_from_memory(&img_bytes).unwrap();

        let tracklist = record_data["tracklist"]
            .as_array()
            .unwrap()
            .iter()
            .map(|track| Track {
                title: track["title"].as_str().unwrap().to_string(),
                duration: track["duration"].as_str().unwrap().to_string(),
                position: track["position"].as_str().unwrap().to_string(),
            })
            .collect();

        Ok(Record {
            title: record_data["title"].as_str().unwrap().to_string(),
            artists,
            year: record_data["year"].as_u64().unwrap() as u16,
            genre,
            style,
            country: record_data["country"].as_str().unwrap().to_string(),
            format: format_str,
            image: AsciiArt::<45, 20>::from_image(image)?,
            tracklist,
        })
    }
}

// This removes any "(X)" from the artist name that discogs appends when there
// is more than one artist with the same name
fn process_artist(artist: serde_json::Value) -> String {
    let mut artist = artist.as_str().unwrap().to_string();
    if artist.chars().nth(artist.len() - 1).unwrap() == ')' {
        artist.truncate(artist.len() - 4);
    }
    artist
}
