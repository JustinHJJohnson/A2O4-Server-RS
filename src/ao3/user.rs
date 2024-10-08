use reqwest::blocking::Client;
use scraper::{Html, Selector};

pub struct User {
    username: String,
    password: String,
    auth_token: String,
    pub client: Client,
}

impl User {
    pub fn new(username: &str, password: &str) -> Self {
        println!("logging in");
        let client = Client::builder().cookie_store(true).build().unwrap();

        let html_content = client
            .get("https://archiveofourown.org/users/login")
            .send()
            .unwrap()
            .text();
        let login_page = Html::parse_document(&html_content.unwrap());
        let auth_selector = Selector::parse("input[name=authenticity_token]").unwrap();
        let auth_token: &str = login_page
            .select(&auth_selector)
            .next()
            .unwrap()
            .value()
            .attr("value")
            .unwrap();
        //println!("{}", auth_token);
        let form_data = [
            ("user[login]", username),
            ("user[password]", password),
            ("authenticity_token", auth_token),
        ];
        let login_response = client
            .post("https://archiveofourown.org/users/login")
            .form(&form_data)
            .send()
            .unwrap();
        // TODO do error checking here on the response status
        //println!("{:?}", login_response.status());
        println!("Successfully logged in\n");

        Self {
            username: username.to_owned(),
            password: password.to_owned(),
            auth_token: auth_token.to_owned(),
            client,
        }
    }
}
