use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io;

const BASE_URL: &str = "tg.com";

fn main() {
    let mut db: HashMap<String, String> = HashMap::new();
    loop {
        println!("1) Add a URL");
        println!("2) Get a URL");
        println!("3) Print URLs");
        println!("4) Exit");
        println!("Select an option:");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let choice: u8 = match input.trim().parse() {
            Ok(num) => num,
            _ => {
                println!("Please enter a valid number\n");
                continue;
            }
        };

        match choice {
            1 => {
                println!("Input a URL:");
                let mut url = String::new();
                io::stdin()
                    .read_line(&mut url)
                    .expect("Failed to read line");
                add_url(&mut db, url.trim());
                println!("Shortened to {}/{}\n", BASE_URL, url.trim());
            }
            2 => {
                if db.is_empty() {
                    println!("Database is empty\n");
                    continue;
                }

                println!("Input a URL:");
                let mut url = String::new();
                io::stdin()
                    .read_line(&mut url)
                    .expect("Failed to read line");
                println!("{}\n", get_url(&db, url.as_str().trim()));
            }
            3 => {
                if db.is_empty() {
                    println!("Database is empty\n");
                    continue;
                }

                list_urls(&db);
                println!();
            }
            4 => {
                println!("Exiting");
                break;
            }
            _ => {
                println!("Invalid option\n");
            }
        }
    }
}

// DB operations
fn add_url(db: &mut HashMap<String, String>, url: &str) {
    db.insert(encode(url), url.to_string());
}

fn get_url(db: &HashMap<String, String>, url: &str) -> String {
    // grab the last item in the url (encoded strings we're using as keys)
    let key = url.split('/').next_back().unwrap().to_string();
    match db.get(&key) {
        Some(s) => s.to_string(),
        _ => {
            println!("Key not found!");
            "".to_string()
        }
    }
}

fn list_urls(db: &HashMap<String, String>) {
    for (k, v) in db.iter() {
        println!("{}/{}, {}", BASE_URL, k, v);
    }
}

// fn shorten_url(url: &str) -> String {
//     let mut s = String::from(BASE_URL);
//     s.push('/');
//     s.push_str(encode(url).as_str());
//     s
// }

fn encode(s: &str) -> String {
    let hash = Sha256::digest(s);
    let bytes = hash.as_slice();
    let eight_bytes: [u8; 8] = bytes[..8].try_into().unwrap();
    let number = u64::from_be_bytes(eight_bytes);
    base62::encode(number)
}
