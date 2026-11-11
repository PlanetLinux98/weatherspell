// What Check for Updates would answer, from the real list, without the
// app: `cargo run -p weatherspell-core --features net --example update --
// 0.1.0 [--previews] [--file Weatherspell.dmg] [--download <folder>]`.
// With --download the update's file is fetched into the folder and
// checked against the release's SHA256SUMS; nothing is installed or run.

use weatherspell_core::net::UreqFetch;
use weatherspell_core::updates::{self, Answer, Version, wording};

fn main() {
    let mut running = None;
    let mut previews = false;
    let mut file = updates::WINDOWS_FILE.to_string();
    let mut folder = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--previews" => previews = true,
            "--file" => file = arguments.next().expect("a file name after --file"),
            "--download" => folder = Some(arguments.next().expect("a folder after --download")),
            version => running = Version::parse(version),
        }
    }
    let running = running.expect("the running version, such as 0.2.0");
    let feed = std::env::var("WEATHERSPELL_UPDATE_FEED")
        .unwrap_or_else(|_| updates::RELEASES_URL.to_string());
    let http = UreqFetch::new("Weatherspell-example");

    let update = match updates::look(&http, &feed, &running, previews, &file) {
        Err(problem) => return println!("{}", wording::could_not_check(&problem)),
        Ok(Answer::Latest) => return println!("{}", wording::latest(&running)),
        Ok(Answer::Ahead { official }) => {
            return println!("{}", wording::ahead(&running, &official));
        }
        Ok(Answer::Newer(update)) => update,
    };
    println!("{}", wording::available(&update, &running));
    println!("\n{}\n", update.notes);
    println!("{} ({} bytes)", update.file.url, update.file.size);

    let Some(folder) = folder else { return };
    let to = std::path::Path::new(&folder).join(format!("{file}.new"));
    println!("{}", wording::downloading(&update));
    let mut last = 0;
    let result = updates::download(&http, &update, &to, &mut |received| {
        let percent = received * 100 / update.file.size.max(1);
        if percent >= last + 10 {
            last = percent - percent % 10;
            println!("{last} percent");
        }
        true
    });
    match result {
        Ok(()) => println!("Matches its checksum: {}", to.display()),
        Err(problem) => println!("{problem}"),
    }
}
