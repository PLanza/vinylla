// This is my application's consumer key and secret which for obvious reasons are not included in
// the repository. If you wish to extend the project you'll have to create your own Discogs
// developer tokens, which is linked here: https://www.discogs.com/developers#page:authentication
use crate::config::{CONSUMER_KEY, CONSUMER_SECRET};

use crossterm::{cursor, execute, terminal};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Struct containing the Discogs APIs user authentication tokens
// These get serialized and saved to a file after logging in to keep the user's session across app
// startups
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct UserData {
    pub oauth_token: String,
    pub oauth_token_secret: String,
}

// Login and API requests require 3 request types that are formatted differently
pub enum RequestType {
    RequestURL,
    PostAccess,
    RequestAuthorized,
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

// Creates the headers necessary to make a GET request to the Discogs API
pub fn create_headers(
    request_type: RequestType,
    oauth_token: Option<String>,
    oauth_token_secret: Option<String>,
    verifier: Option<&str>,
) -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

    // This holds the HTTP GET requests' headers needed to make the requests according to the
    // Discogs API
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse().unwrap(),
    );

    // The auth_string is appended to depending on the request type
    let mut auth_string = format!(
        "OAuth \
            oauth_consumer_key=\"{0}\", \
            oauth_nonce=\"{1}\", \
            oauth_signature_method=\"PLAINTEXT\", \
            oauth_timestamp=\"{1}\", \
            ",
        CONSUMER_KEY,
        get_timestamp()
    );

    // Adds the necessary information to the auth_string depending on the request type as explained
    // here: https://www.discogs.com/developers#page:authentication
    match request_type {
        RequestType::RequestURL => {
            auth_string.push_str(format!("oauth_signature=\"{}&\"", CONSUMER_SECRET).as_str());
        }
        RequestType::PostAccess => {
            auth_string.push_str(
                format!(
                    "oauth_token=\"{}\", \
                oauth_signature=\"{}&{}\", \
                oauth_verifier=\"{}\"",
                    oauth_token.unwrap(),
                    CONSUMER_SECRET,
                    oauth_token_secret.unwrap(),
                    verifier.unwrap()
                )
                .as_str(),
            );
        }
        RequestType::RequestAuthorized => {
            auth_string.push_str(
                format!(
                    "oauth_token=\"{}\", \
                oauth_signature=\"{}&{}\"",
                    oauth_token.unwrap(),
                    CONSUMER_SECRET,
                    oauth_token_secret.unwrap(),
                )
                .as_str(),
            );
        }
    }

    let auth_string = auth_string.as_bytes();
    headers.insert(AUTHORIZATION, HeaderValue::from_bytes(auth_string).unwrap());
    headers.insert(USER_AGENT, "Vinylla/0.1".parse().unwrap());

    headers
}

// Authenicates a user following the authentication process outlined on the Discogs API page
// This function is called when the user executes the 'Login' command
pub(crate) fn authenticate(client: &Client) -> Result<UserData, reqwest::Error> {
    let response = client
        .get("https://api.discogs.com/oauth/request_token")
        .headers(create_headers(RequestType::RequestURL, None, None, None))
        .send()?
        .text()?;

    // Retrieve the authentication tokens from the GET response
    let mut oauth_token = response.replace("oauth_token=", "");
    oauth_token.truncate(oauth_token.find("&oauth_token_secret").unwrap());
    let oauth_token_secret = response.as_str()
        [(response.find("&oauth_token_secret=").unwrap() + "&oauth_token_secret=".len())..]
        .to_string();

    // Prompts the user to authorize the application on their browser through a link...
    print!("║ Please authorize the application at the link below.                                                                            ║\r\n");
    print!(
        "║ https://discogs.com/oauth/authorize?oauth_token={:79}║",
        oauth_token
    );
    print!("║ Then paste the code here:                                                                                                      ║\r\n");
    execute!(std::io::stdout(), cursor::MoveTo(28, 36)).unwrap();
    terminal::disable_raw_mode().unwrap();

    // ... retrieving user input after disabling raw mode to paste in the code
    let mut verifier = String::new();
    std::io::stdin().read_line(&mut verifier).unwrap();

    execute!(std::io::stdout(), cursor::MoveTo(0, 37)).unwrap();
    terminal::enable_raw_mode().unwrap();

    // Then sends another GET request to the api with the user's code..
    let response = client
        .post("https://api.discogs.com/oauth/access_token")
        .headers(create_headers(
            RequestType::PostAccess,
            Some(oauth_token),
            Some(oauth_token_secret),
            Some(verifier.trim_end()),
        ))
        .send()?
        .text()?;

    // ... to then retrieve the users authentication tokens
    let mut oauth_token = response.replace("oauth_token=", "");
    oauth_token.truncate(oauth_token.find("&oauth_token_secret").unwrap());
    let oauth_token_secret = response.as_str()
        [(response.find("&oauth_token_secret=").unwrap() + "&oauth_token_secret=".len())..]
        .to_string();

    let user_data = UserData {
        oauth_token,
        oauth_token_secret,
    };

    Ok(user_data)
}

// A utility function to more easily make an authenticated request
pub(crate) fn make_auth_request(client: &Client, user_data: &UserData, url: String) -> reqwest::Result<String> {
        let response = client 
            .get(url)
            .headers(create_headers(
                RequestType::RequestAuthorized,
                Some(user_data.oauth_token.clone()),
                Some(user_data.oauth_token_secret.clone()),
                None,
            ))
            .send()?
            .text()?;

        Ok(response)
    }

