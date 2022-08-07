use crate::discogs_client::{authenticate, UserData, make_auth_request};
use crate::record::{load_collection, Record, RecordCollection};

use crossterm::{cursor, event, execute, style::Stylize, terminal};
use reqwest::blocking::Client;
use std::io::{stdout, Result};
use std::path::Path;

const APP_COLS: u16 = 130;
const APP_ROWS: u16 = 40;
// Path to user's collection data
const USER_DATA_PATH: &str = "data/user_data.json";

// user_data: The user's Discogs authentication keys
// client: A blocking HTTP client to make requests to the Discogs API
// selected: The index of the currently selected record
// collection: The user's record collection data
// sorted_titles: The collection's (artist, title) pair sorted as is displayed in the app
pub struct App {
    user_data: Option<UserData>,
    pub(crate) client: Client,
    selected: usize,
    collection: RecordCollection,
    sorted_titles: Vec<(String, String)>,
}

impl App {
    pub fn init() -> Result<App> {
        let mut user_data = None;
        // Load the user's authentication tokens if they have logged in previously
        if Path::new(USER_DATA_PATH).exists() {
            let data_string = std::fs::read_to_string(USER_DATA_PATH)?;
            user_data = Some(serde_json::from_str(data_string.as_str())?);
        }

        let collection = load_collection()?;

        // Create a vector of sorted titles from the collection that can be quickly referenced
        let sorted_titles: Vec<&(String, String)> = collection.keys().collect();
        let mut sorted_titles: Vec<(String, String)> =
            sorted_titles.into_iter().map(|k| k.clone()).collect();
        sorted_titles.sort();

        // Raw mode changes the terminal's behavior
        // For example, ignores Ctrl-C and doesn't write keyboard input to the terminal
        terminal::enable_raw_mode()?;

        // Resizes the terminal to the app's size (130 x 40) 
        let old_term_size = terminal::size()?;
        execute!(stdout(), terminal::SetSize(APP_COLS, APP_ROWS))?;

        // Pauses execution until terminal is resized
        if old_term_size != (APP_COLS, APP_ROWS) {
            wait_for_resize()?;
        }

        execute!(stdout(), cursor::Hide)?;

        Ok(App {
            user_data,
            client: Client::new(),
            selected: 0,
            collection,
            sorted_titles,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        use crossterm::event::{
            read,
            Event::{Key, Resize},
            KeyCode, KeyEvent,
        };

        // The main run loop
        loop {
            self.print()?;
            match read()? {
                Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') => {
                        self.command_mode()?;
                    }
                    // Moves selection up and down, within 0..collection.len() bounds
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down => {
                        self.selected = (self.selected + 1).min(self.collection.len() - 1)
                    }
                    _ => (),
                },
                // Prevents user from resizing app since printing is dependent on a set size
                // Resets the terminal to the application size when the user resizes it
                // Doesn't work when full screen, or sticky to the side of the screen
                Resize(..) => {
                    execute!(stdout(), terminal::SetSize(APP_COLS, APP_ROWS))?;
                    wait_for_resize()?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn print(&self) -> Result<()> {
        execute!(stdout(), cursor::MoveTo(0, 1))?;
        // Print Header
        print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");
        print!("║                                                        Vinylla - v0.1.0                                                        ║\r\n");
        print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        
        // Print Contents
        self.print_content(APP_ROWS - 1 as u16 - 6)?;

        // Print Footer
        execute!(stdout(), cursor::MoveTo(0, 37))?;
        print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");
        print!("║ {}ommand:                                                                                                                       ║\r\n",
            "C".underlined()
        );
        print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        Ok(())
    }

    fn print_content(&self, rows: u16) -> Result<()> {
        execute!(stdout(), cursor::MoveTo(0, 4))?;
        print!("╔══════════════════════════════════════╗ ╔═══════════════════════════════════════════════════════════════════════════════════════╗\r\n");
        
        // Gets the artist and title of the selected record to display at the info header
        let title_str = if self.sorted_titles.len() != 0 {
            format!(
                "{} - {}",
                self.sorted_titles[self.selected].0,
                self.sorted_titles[self.selected].1
            )
        } else {
            "".to_string()
        };
        
        // Holds the information of the currently selected record
        // record is None if there aren't any records in the collection
        let record = if self.sorted_titles.len() != 0 {
            Some(
                self.collection
                    .get(&self.sorted_titles[self.selected])
                    .unwrap(),
            )
        } else {
            None
        };

        print!(
            "║              My Records              ║ ║ {:^85} ║\r\n",
            title_str
        );
        print!("╟──────────────────────────────────────╢ ╟───────────────────────────────────────────────────────────────────────────────────────╢\r\n");

        // Is set to true once the iterator reaches the end of the collection
        let mut reached_end = self.sorted_titles.len() == 0;
        // Prints the content section row by row
        for i in 1..(rows - 3) {
            execute!(stdout(), cursor::MoveTo(0, 6 + i))?;
            // Prints the left section with the records listing
            if !reached_end {
                // record_str holds the string for a record in the listing on the left
                // TODO: handle printing when selected record's index is greater than then number
                //       of rows
                let mut record_str = if self.selected == i as usize - 1 {
                    format!("> {}. ", i)
                } else {
                    format!("  {}. ", i)
                };

                // Appends the record artist and title to the string
                match self.sorted_titles.get(i as usize - 1) {
                    Some((artist, title)) => {
                        record_str.push_str(artist.as_str());
                        record_str.push_str(" - ");
                        record_str.push_str(title.as_str());
                    }
                    None => {
                        reached_end = true;
                        record_str = String::new();
                    }
                }
                // Truncates the string to be within the sections bounds
                max_len(&mut record_str, 35);

                print!("║{:37} ║ ", record_str);
            } else {
                print!("║  {:35} ║ ", "");
            }

            // If a record is selected (collection is not empty) print a row of the info section
            match record {
                Some(record) => self.print_info_row(i, &record),
                // Otherwise print a blank row
                None => print!("║ {:^85} ║\r\n", ""),
            }
        }

        execute!(stdout(), cursor::MoveTo(0, 3 + rows))?;
        print!("╚══════════════════════════════════════╝ ╚═══════════════════════════════════════════════════════════════════════════════════════╝\r\n");

        // Print the selected record's album cover and tracklist 
        match record {
            Some(record) => {
                record.image.print_at((82, 8))?;
                print_tracklist(20, record)?;
            }
            None => (),
        }

        Ok(())
    }

    // Takes the selected record and a row to print and prints the corresponding information
    fn print_info_row(&self, i: u16, record: &Record) {
        match i {
            2 => print!(
                "║   {:9}{:<24}   {:41}  ║\r\n",
                "Release:",
                record.year.to_string(),
                ""
            ),
            4 => print!(
                "║   {:9}{:<24}   {:41}  ║\r\n",
                "Genre:",
                max_len(&mut record.genre.join(" / "), 24),
                ""
            ),
            6 => print!(
                "║   {:9}{:<24}   {:41}  ║\r\n",
                "Style:",
                max_len(&mut record.style.join(" / "), 24),
                ""
            ),
            8 => print!(
                "║   {:9}{:<24}   {:41}  ║\r\n",
                "Country:", record.country, ""
            ),
            10 => print!(
                "║   {:9}{:<24}   {:41}  ║\r\n",
                "Format:", record.format, ""
            ),
            12 => print!("║   {:^34}   {:41}  ║\r\n", "Tracklist", ""),
            13 => print!("║   {:^34}   {:41}  ║\r\n", "─────────────────────", ""),
            _ => print!("║ {:^85} ║\r\n", ""),
        }
    }

    // Handles command mode
    fn command_mode(&mut self) -> Result<()> {
        execute!(stdout(), cursor::MoveTo(11, 37), cursor::Show)?;
        // Disables raw mode so that the use can freely enter a command
        terminal::disable_raw_mode()?;

        // Read command from user
        let mut command = String::new();
        std::io::stdin().read_line(&mut command)?;
        let command = command.trim_end();

        match command {
            "Login" => self.login()?,
            "Add" => self.add_record()?,
            "Remove" => self.remove_selected()?,
            _ => (),
        }

        // Enable raw mode and resume regular print loop 
        terminal::enable_raw_mode()?;
        execute!(stdout(), cursor::Hide)?;

        Ok(())
    }

    // Handles user login
    fn login(&mut self) -> Result<()> {
        // Prints prompt box
        execute!(stdout(), cursor::MoveTo(0, 32))?;
        print!("╚══════════════════════════════════════╝ ╚═══════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");
        print_blank_lines(4);
        print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        execute!(stdout(), cursor::MoveTo(0, 34))?;

        // Retrieves user authentication tokens needed to make authenticated requests
        let user_data = authenticate(&self.client).unwrap();
        // Saves (and overwrites it) to a data file
        let data_string = serde_json::to_string(&user_data)?;
        std::fs::write("data/user_data.json", data_string)?;
        self.user_data = Some(user_data);

        print!("║ Login Successful!                                                                                                              ║\r\n");
        execute!(stdout(), cursor::Hide)?;
        wait_for_enter()?;

        Ok(())
    }

    // Handles adding a new record to the collection
    fn add_record(&mut self) -> Result<()> {
        match &self.user_data {
            // Authenticated requests are needed to retrieve image urls and search the database
            None => {
                // Prints prompt box
                execute!(stdout(), cursor::MoveTo(0, 36))?;
                print!("╚══════════════════════════════════════╝ ╚═══════════════════════════════════════════════════════════════════════════════════════╝\r\n");
                print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");
                print!("║ You need to log into a Discogs account with the 'Login' command before adding a record to your collection.                     ║\r\n");
                print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
                wait_for_enter()?;

            },
            Some(user_data) => {
                // Prints prompt box
                execute!(stdout(), cursor::MoveTo(0, 31))?;
                print!("╚══════════════════════════════════════╝ ╚═══════════════════════════════════════════════════════════════════════════════════════╝\r\n");
                print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");
                print!("║ Enter the details of the record you want to add to your collection.                                                            ║\r\n");
                print_blank_lines(1);
                print!("║ Artist:                                                                                                                        ║\r\n");
                print!("║ Album:                                                                                                                         ║\r\n");
                print_blank_lines(1);
                print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
                execute!(stdout(), cursor::MoveTo(10, 35))?;

                terminal::disable_raw_mode().unwrap();

                // Retrieves user input
                let mut artist = String::new();
                std::io::stdin().read_line(&mut artist).unwrap();

                execute!(std::io::stdout(), cursor::MoveTo(9, 36)).unwrap();
                let mut album = String::new();
                std::io::stdin().read_line(&mut album).unwrap();

                terminal::enable_raw_mode().unwrap();

                // Forms database url given the user information, limitting the search to master releases
                let search_url = format!(
                    "https://api.discogs.com/database/search?q={}-{}&type=master",
                    process_search_string(artist),
                    process_search_string(album)
                );

                // Gets the results of searching
                let search_data = make_auth_request(&self.client, user_data, search_url).unwrap();
                let search: serde_json::Value = serde_json::from_str(&search_data.as_str())?;

                // Gets the information from the master release (doesn't contain tracklist, country, etc.)
                let master_data = make_auth_request(&self.client, user_data, search["results"][0]["master_url"].as_str().unwrap().into())
                    .unwrap();
                let master: serde_json::Value = serde_json::from_str(&master_data)?;

                // Gets the information from the main release 
                let release_data = make_auth_request(&self.client, user_data, master["main_release_url"].as_str().unwrap().into())
                    .unwrap();
                let main_release: serde_json::Value = serde_json::from_str(&release_data)?;

                // Creates a Record struct from the main release's information
                let new_record = Record::from_discogs(main_release)?;
                let key = (new_record.artists[0].clone(), new_record.title.clone());

                // Shifts the selected index to not be affected by the new addition
                if self.sorted_titles.len() != 0 && self.sorted_titles[self.selected] > key {
                    self.selected += 1;
                }

                // Adds record to the collection...
                match self.collection.insert(key.clone(), new_record) {
                    Some(_) => (),
                    None => {
                        // ... and to the sorted_titles Vec if it's new
                        self.sorted_titles.push(key);
                        self.sorted_titles.sort();
                    }
                }

                print!("║ Record added to your collection!                                                                                              ║\r\n");

                execute!(stdout(), cursor::Hide)?;
                wait_for_enter()?;
            }
        }

        Ok(())
    }

    // Handles removing the selected record from the collection
    fn remove_selected(&mut self) -> Result<()> {
        // Prompt string
        let mut remove_str = format!(
            "Are you sure you want to delete {} by {} from your collection (y/n)? ",
            self.sorted_titles[self.selected].1,
            self.sorted_titles[self.selected].0
        );

        // Trims it to fit within the prompt boxes
        max_len(&mut remove_str, 126);

        execute!(stdout(), cursor::MoveTo(0, 35))?;
        print!("╚══════════════════════════════════════╝ ╚═══════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        print!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\r\n");

        print!("║ {:126} ║\r\n", remove_str);
        print!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝\r\n");
        execute!(stdout(), cursor::MoveTo(2 + remove_str.len() as u16, 37))?;

        terminal::disable_raw_mode().unwrap();

        // Retrieves user input
        let mut response = String::new();
        std::io::stdin().read_line(&mut response).unwrap();
        let response = response.trim();

        terminal::enable_raw_mode().unwrap();

        match response {
            "y" | "yes" | "Y" | "Yes" => {
                // Removes record from both the collection and the sorted_titles list
                self.collection.remove(&self.sorted_titles[self.selected]);
                self.sorted_titles.remove(self.selected);
                self.selected = self.selected.min(self.sorted_titles.len() - 1);

                execute!(stdout(), cursor::MoveTo(0, 37))?;
                print!("║ Record removed from collection!                                                                                        ║\r\n");
            }
            _ => print!("║ Cancelled removal of record.                                                                                           ║\r\n"),
        }
        execute!(stdout(), cursor::Hide)?;
        wait_for_enter()?;

        Ok(())
    }

    // Quits the application after running it 
    pub fn quit(self) -> Result<()> {
        // Disables raw mode
        terminal::disable_raw_mode()?;

        // And writes collection data to a file so that it can be retrieved on startup
        let records = self.collection.into_values().collect::<Vec<Record>>();
        let collection_string = serde_json::to_string(&records)?;
        std::fs::write("data/collection.json", collection_string)?;

        Ok(())
    }
}

// Loops until a resize occurs
fn wait_for_resize() -> Result<()> {
    loop {
        match event::read()? {
            event::Event::Resize(..) => break,
            _ => (),
        }
    }
    Ok(())
}

// Loops until the Enter key is pressed
fn wait_for_enter() -> Result<()> {
    use crossterm::event::{
            read,
            Event::Key,
            KeyCode,
            KeyEvent
        };

    loop {
        match read()? {
            Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Enter => break,
                    _ => (),
            },
            _ => (),
        }
    }
    Ok(())

}

// Prints n blanks section rows with boundaries corresponding to APP_COLS
fn print_blank_lines(n: u32) {
    for _ in 0..n {
        print!("║                                                                                                                                ║\r\n");
    }
}

// Takes user's artist and title input and returns a string that can be appended to the search url
// For example: "stan getz / joao gilberto" -> "Stan+Getz+Joao+Gilberto"
fn process_search_string(s: String) -> String {
    let v: Vec<&str> = s.trim().split(' ').collect();
    let mut w: Vec<String> = Vec::new();
    for s in v.iter() {
        // Removes non alphanumeric characters e.g. " / " or " - "
        if s.len() == 1 && !s.to_string().chars().nth(0).unwrap().is_alphanumeric() {
            continue;
        } else {
            // Capitalizes the first letter of each word
            let mut c = s.chars();
            match c.next() {
                None => w.push(String::new()),
                Some(f) => {
                    let capitalized = f.to_uppercase().collect::<String>() + c.as_str();
                    w.push(capitalized);
                }
            }
        }
    }

    // Joins the processed words with a '+'
    w.join("+")
}

// Truncates a given string to len, appending "..." at the end
fn max_len(string: &mut String, len: usize) -> &mut String {
    if string.len() > len {
        string.truncate(len - 3);
        string.push_str("...");
    }

    string
}

// Prints a given record's tracklist at a maximum of 15 rows
// start_row is not really necessary as it is always printed starting from the same row
fn print_tracklist(start_row: u16, record: &Record) -> Result<()> {
    let mut row = 0;
    let mut sides: Vec<String> = Vec::new();
    let mut track = 0;
    // Iterates until either the end of the tracklist or 15 rows have been drawn
    while track < record.tracklist.len() && row < 15 {
        execute!(stdout(), cursor::MoveTo(45, start_row + row))?;
        let current_track = &record.tracklist[track];
        // Extracts the side name (A, B, etc.) from the track data
        let side = current_track.position.get(0..1).unwrap().to_string();

        // If it's a new side, print a "Side X:" header...
        if !sides.contains(&side) {
            if sides.len() != 0 {
                row += 1;
                execute!(stdout(), cursor::MoveTo(45, start_row + row))?;
            }
            sides.push(side.clone());
            print!("Side {}:", side);
        } else {
            // Otherwise, print the track number, title and duration
            // Ignore track number if it is not given
            let position = current_track.position.get(1..).unwrap();
            let mut track_str = if position.len() == 0 {
                format!("  {:25} {}", current_track.title, current_track.duration)
            } else {
                format!(
                    "  {}. {:23} {}",
                    position, current_track.title, current_track.duration
                )
            };

            // Trim the track_str to fit in the info box
            if track_str.len() > 33 {
                let end = track_str.len() - (current_track.duration.len() + 1);
                let begin = 33 - (current_track.duration.len() + 4);
                track_str.replace_range(begin..end, "...")
            }

            print!("{}", track_str);
            track += 1;
        }
        row += 1;
    }

    Ok(())
}
