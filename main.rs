use std::env;
use std::process;

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

struct StartArgs {
    isbn: String,
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
            isbn: isbn.ok_or("missing --isbn")?,
            authors: authors.ok_or("missing --authors")?,
            date: date.ok_or("missing --date")?,
            title: title.ok_or("missing --title")?
        })
    }
}

fn start(args: StartArgs) {
    println!("{}", args.isbn);
    println!("{}", args.date);
    println!("{}", args.authors.join(", "));
    println!("{}", args.title);
}

struct RateArgs {
    id: i8,
    rating: i8,
    progress: i8,
    notes: String,
    date: String
}

impl RateArgs {
    fn build(args: &[String]) -> Result<RateArgs, &'static str> {
        let mut id = None;
        // let mut notes = None;
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
                _ => return Err("unknown arg")
            }
        }
        Ok(RateArgs {
            id: id.ok_or("missing --id")?,
            rating: rating.ok_or("missing --rating")?,
            progress: progress,
            notes: "TODO".to_string(),
            date: date
        })
    }
}

fn rate(args: RateArgs) {
    println!("{}", args.id);
    println!("{}", args.rating);
    println!("{}", args.notes);
    println!("{}", args.date);
    println!("{}", args.progress);
}

enum Subcommand {
    List,
    Rate(RateArgs),
    Start(StartArgs)
}

impl Subcommand {
    fn build(args: &[String]) -> Result<Subcommand, &'static str> {
        match args[1].as_str() {
            "ls" => Ok(Subcommand::List),
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
        Subcommand::List => println!("ls"),
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
