use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use crate::config::Config;

pub struct User {
    username: String,
    password: String,
    auth_token: String,
    pub client: Client,
}

impl User {
    //TODO AO3 has a banner if already logged in, maybe useful login caching
    pub async fn new(username: &str, password: &str) -> Self {
        println!("logging in");
        let client = Client::builder().cookie_store(true).build().unwrap();

        let html_content = client
            .get("https://archiveofourown.org/users/login")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let regex = Regex::new(r#"(id="new_user").+?("authenticity_token").+?"(?<token>.+?)""#).unwrap();
        let auth_token: &str= &regex.captures(&html_content).unwrap()["token"];
        let form_data = [
            ("user[login]", username),
            ("user[password]", password),
            ("authenticity_token", auth_token),
        ];
        let login_response = client
            .post("https://archiveofourown.org/users/login")
            .form(&form_data)
            .send()
            .await
            .unwrap();
        // TODO do error checking here on the response status
        println!("{:?}", login_response.status());
        println!("Successfully logged in\n");

        Self {
            username: username.to_owned(),
            password: password.to_owned(),
            auth_token: auth_token.to_owned(),
            client,
        }
    }
}

pub async fn get_user(config: &Config) -> Option<User> {
    if let (Some(username), Some(password)) = (&config.ao3_username, &config.ao3_password) {
        Some(User::new(username, password).await)
    } else {
        None
    }
}
