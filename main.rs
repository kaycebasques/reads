use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use serde::{Deserialize, Serialize};

// helpers

fn data_path() -> PathBuf {
    match env::var("BUILD_WORKSPACE_DIRECTORY") {
        Ok(dir) => PathBuf::from(dir).join("data.json"),
        Err(_) => PathBuf::from("data.json"),
    }
}

fn today() -> String {
    let result = process::Command::new("date")
        .arg("+%Y%m%d")
        .output()
        .expect("could not get date");
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}

fn parse_date(val: &str) -> Result<String, &'static str> {
    if val.len() == 8 && val.chars().all(|c| c.is_ascii_digit()) {
        Ok(val.to_string())
    } else {
        Err("invalid date (expected YYYYMMDD)")
    }
}

fn load_books() -> Vec<Book> {
    let raw = fs::read_to_string(data_path()).expect("data.json read failed");
    serde_json::from_str(&raw).expect("data.json parse failed")
}

fn save_books(books: &[Book]) {
    let out = serde_json::to_string_pretty(books).expect("data.json serialization failed");
    fs::write(data_path(), out).expect("data.json write failed");
}

fn find_book<'a>(
    books: &'a mut [Book],
    isbn: Option<&str>,
    title: &str
) -> Option<&'a mut Book> {
    for book in books {
        if book.matches(isbn, title) {
            return Some(book);
        }
    }
    None
}

// core data structures

#[derive(Serialize, Deserialize)]
struct Session {
    start: Option<String>,
    end: Option<String>,
    notes: Option<String>,
    rating: Option<u8>,
    progress: Option<u8>
}

impl Session {
    fn new (start: String) -> Self {
        Self {
            start: Some(start),
            end: None,
            notes: None,
            rating: None,
            progress: None
        }
    }
    fn is_active(&self) -> bool {
        self.start.is_some() && self.end.is_none()
    }
}

#[derive(Serialize, Deserialize)]
struct Book {
    isbn: Option<String>,
    title: String,
    authors: Vec<String>,
    sessions: Vec<Session>
}

impl Book {
    fn matches(&self, isbn: Option<&str>, title: &str) -> bool {
        match (self.isbn.as_deref(), isbn) {
            (Some(a), Some(b)) => a == b,
            _ => self.title.eq_ignore_ascii_case(title),
        }
    }

    fn is_active(&self) -> bool {
        self.sessions.iter().any(Session::is_active)
    }

    fn active_session_mut(&mut self) -> Option<&mut Session> {
        for session in &mut self.sessions {
            if session.is_active() {
                return Some(session);
            }
        }
        None
    }
}

// start subcommand

struct StartArgs {
    isbn: Option<String>,
    authors: Vec<String>,
    date: String,
    title: String
}

impl StartArgs {
    fn build(args: &[String]) -> Result<StartArgs, &'static str> {
        let mut isbn = None;
        let mut authors = None;
        let mut date = None;
        let mut title = None;
        for arg in args {
            let (key, val) = arg
                .strip_prefix("--")
                .and_then(|s| s.split_once('='))
                .ok_or("expected --key=value")?;
            match key {
                "isbn" => isbn = Some(val.to_string()),
                "authors" => authors = Some(
                    val.split(',').map(|s| s.trim().to_string()).collect()
                ),
                "date" => date = Some(parse_date(val)?),
                "title" => title = Some(val.to_string()),
                _ => return Err("unknown arg")
            }
        }
        Ok(StartArgs {
            isbn: isbn,
            authors: authors.ok_or("missing --authors")?,
            date: date.ok_or("missing --date")?,
            title: title.ok_or("missing --title")?
        })
    }
}

fn start(args: StartArgs) {
    let mut books = load_books();
    let session = Session::new(args.date);
    if let Some(book) = find_book(&mut books, args.isbn.as_deref(), &args.title) {
        if book.is_active() {
            eprintln!("book already active");
            process::exit(1);
        }
        book.sessions.push(session);
        println!("book found, starting new session");
        save_books(&books);
        return;
    }
    println!("creating first session for new book");
    books.push(Book {
        isbn: args.isbn,
        title: args.title,
        authors: args.authors,
        sessions: vec![session]
    });
    save_books(&books);
}

// rate subcommand

struct RateArgs {
    id: usize,
    rating: u8,
    progress: u8,
    notes: Option<String>,
    date: String
}

impl RateArgs {
    fn build(args: &[String]) -> Result<RateArgs, &'static str> {
        let mut id = None;
        let mut notes = None;
        let mut rating = None;
        let mut progress = 100;
        let mut date = today();
        for arg in args {
            let (key, val) = arg
                .strip_prefix("--")
                .and_then(|s| s.split_once('='))
                .ok_or("expected --key=value")?;
            match key {
                "id" => id = Some(val.parse().map_err(|_| "invalid id")?),
                "rating" => rating = Some(val.parse().map_err(|_| "invalid rating")?),
                "progress" => progress = val.parse().map_err(|_| "invalid progress")?,
                "date" => date = parse_date(val)?,
                "notes" => notes = Some(val.to_string()),
                _ => return Err("unknown arg")
            }
        }
        Ok(RateArgs {
            id: id.ok_or("missing --id")?,
            rating: rating.ok_or("missing --rating")?,
            progress,
            notes,
            date
        })
    }
}

fn rate(args: RateArgs) {
    let mut books = load_books();
    let book = books.get_mut(args.id).expect("invalid book id");
    let session = book.active_session_mut().expect("no active session found");
    session.end = Some(args.date);
    session.rating = Some(args.rating);
    session.progress = Some(args.progress);
    session.notes = args.notes;
    save_books(&books);
    println!("saved rating ({}) for {}", args.rating, books[args.id].title);
}

// ls subcommand

fn ls() {
    let books = load_books();
    for (id, book) in books.iter().enumerate().filter(|(_, b)| b.is_active()) {
        println!("[{id}] {}", book.title);
    }
}

// entrypoint stuff

enum Subcommand {
    Ls,
    Rate(RateArgs),
    Start(StartArgs)
}

impl Subcommand {
    fn build(args: &[String]) -> Result<Subcommand, &'static str> {
        match args[1].as_str() {
            "ls" => Ok(Subcommand::Ls),
            "rate" => {
                let args = RateArgs::build(&args[2..])?;
                Ok(Subcommand::Rate(args))
            },
            "start" => {
                let args = StartArgs::build(&args[2..])?;
                Ok(Subcommand::Start(args))
            },
            _ => Err("unknown subcommand")
        }
    }
}

fn run(subcommand: Subcommand) {
    match subcommand {
        Subcommand::Ls => ls(),
        Subcommand::Rate(args) => rate(args),
        Subcommand::Start(args) => start(args),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let subcommand = Subcommand::build(&args).unwrap_or_else(|err| {
        println!("{err}");
        process::exit(1);
    });
    run(subcommand);
}
