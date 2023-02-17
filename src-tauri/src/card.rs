use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;
use std::io::prelude::*;

use serde::{Deserialize, Serialize};

use regex::Regex;

use chrono::{DateTime, Utc};

use crate::deck;
// TODO: Reconsider the interfaces that are used to
// render notes -- should probably just have a function
// that takes a note and returns all the cards generated
// from that note
use crate::note::{Note, get_note_path, parse_note_into_fields, render_back, render_front};
use crate::review::{Review, ReviewState, ReviewScore};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    note_id: String,
    deck_id: String,
    card_num: u32,
    interval: u32,
    due: Option<DateTime<Utc>>,
    ease: u32,
    state: ReviewState,
    steps: u32,
    template: String,
}

pub fn get_review_path(card: Card) -> PathBuf {
    deck::get_deck_path(&card.deck_id)
        .join("reviews")
        .join(format!("{}.jsonl", &card.note_id))
}

impl From<Card> for Note {
    fn from(card: Card) -> Self {
        Note::new(card.note_id, card.deck_id, card.template) 
    }
}

impl From<Card> for Review {
    fn from(card: Card) -> Self {
        Review::new(
                card.due,
                // TODO: Fix these to be f64 up the stack
                card.interval.into(),
                card.ease.into(),
                card.state,
                card.steps,
        )
    }
}

impl Default for Card {
    fn default() -> Self {
        Card {
            note_id: String::from("test"),
            card_num: 1,
            interval: 1,
            ease: 250,
            steps: 0,
            template: String::from("basic"),
            due: None,
            deck_id: String::from("test"),
            state: ReviewState::New,
        }
    }
}

#[tauri::command]
pub fn render_card(card: Card, back: bool) -> Result<String, String> {
    match fs::read_to_string(get_note_path(card.clone().into())) {
        Ok(content) => {
            if back {
                Ok(render_back(
                    parse_note_into_fields(content),
                    card.template,
                    card.card_num,
                ))
            } else {
                Ok(render_front(
                    parse_note_into_fields(content),
                    card.template,
                    card.card_num,
                ))
            }
        }
        Err(err) => Err(err.to_string()),
    }
}


fn get_due_cards_from_paths(deck: &str, paths: ReadDir) -> Vec<Card> {
    let note_filename_regex = Regex::new("([^_]*)?_?(.*).md").unwrap();
    paths
        .filter_map(|path| match path {
            Ok(p) => Some(p),
            Err(_) => None,
        })
        .filter(|x| match x.file_type() {
            Ok(t) => t.is_file(),
            Err(_) => false,
        })
        .map(|path| path.file_name())
        .filter_map(
            |filename| match note_filename_regex.captures(filename.to_str().unwrap()) {
                None => None,
                Some(captures) => {
                    let note_id = captures.get(1).map_or("basic", |x| x.as_str());

                    Some(Card {
                        deck_id: deck.to_string(),
                        card_num: 1,
                        due: Option::None,
                        ease: 200,
                        interval: 100,
                        state: ReviewState::New,
                        steps: 0,
                        template: "basic".to_string(),
                        note_id: note_id.to_string(),
                    })
                }
            },
        )
        .filter(|x| match x.due {
            None => true,
            Some(due) => due < Utc::now(),
        })
        // This is where in the future we'll want to derive other cards based on
        // their templates / cloze deletions
        // we'll also need to parse the filename to get the note id + the template
        .collect()
}

#[tauri::command]
pub fn list_cards_to_review(deck: &str) -> Result<Vec<Card>, String> {
    match fs::read_dir(deck::get_deck_path(deck)) {
        Ok(paths) => Ok(get_due_cards_from_paths(deck, paths)),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
pub fn review_card(card: Card, _score: ReviewScore) -> Result<String, String> {
    match fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(get_review_path(card.clone().into()))
    {
        Ok(mut file) => match file.write_all(&serde_json::to_vec(&card).unwrap()) {
            Ok(..) => Ok("".to_string()),
            Err(..) => Err("".to_string()),
        },
        Err(..) => Err("".to_string()),
    }
}