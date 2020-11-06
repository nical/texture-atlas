#[macro_use]
extern crate serde;

use clap::*;
use texture_atlas::euclid::size2;
use texture_atlas::StatsRecorder;

use std::fs::{File, OpenOptions};
use std::io::prelude::*;

#[derive(Serialize, Deserialize)]
struct Session {
    stats: StatsRecorder,
}

fn main() {
    let matches = App::new("Texture allocation stats command-line interface")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .about("Dynamic texture atlas allocaton stats.")
        .subcommand(
            SubCommand::with_name("init")
            .about("Initialize the atlas")
            .arg(Arg::with_name("WIDTH")
                .help("Rectangle width.")
                .value_name("WIDTH")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("HEIGHT")
                .help("Rectangle height.")
                .value_name("HEIGHT")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("LARGE_SIZE")
                .short("l")
                .long("large")
                .help("Size above which a rectangle is considered large")
                .value_name("LARGE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SMALL_SIZE")
                .short("s")
                .long("small")
                .help("Size above which a rectangle is considered large")
                .value_name("LARGE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SNAP")
                .long("snap")
                .help("Round up the size of the allocated rectangle to a multiple of the provided value.")
                .value_name("SNAP")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("ATLAS")
                .short("a")
                .long("atlas")
                .help("Sets the output atlas file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SVG_OUTPUT")
                .long("svg")
                .help("Dump the atlas in an SVG file")
                .value_name("SVG_OUTPUT")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("allocate")
            .about("Allocate a rectangle")
            .arg(Arg::with_name("WIDTH")
                .help("Rectangle width.")
                .value_name("WIDTH")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("HEIGHT")
                .help("Rectangle height.")
                .value_name("HEIGHT")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("NAME")
                .short("-n")
                .long("name")
                .help("Set a name to identify the rectangle.")
                .value_name("NAME")
                .takes_value(true)
                .required(false)
             )
            .arg(Arg::with_name("ATLAS")
                .short("a")
                .long("atlas")
                .help("Sets the output atlas file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SVG_OUTPUT")
                .long("svg")
                .help("Dump the atlas in an SVG file")
                .value_name("SVG_OUTPUT")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("deallocate")
            .about("De-allocate a rectangle")
            .arg(Arg::with_name("NAME")
                .help("Name of the rectangle to remove.")
                .value_name("NAME")
                .takes_value(true)
                .required(true)
             )
            .arg(Arg::with_name("ATLAS")
                .short("a")
                .long("atlas")
                .help("Sets the output file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SVG_OUTPUT")
                .long("svg")
                .help("Dump the atlas in an SVG file")
                .value_name("SVG_OUTPUT")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("grow")
            .about("Resize the atlas.")
            .arg(Arg::with_name("ATLAS")
                .short("a")
                .long("atlas")
                .help("Sets the output file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("WIDTH")
                .help("New width")
                .value_name("WIDTH")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("HEIGHT")
                .help("New height")
                .value_name("HEIGHT")
                .takes_value(true)
                .required(true)
            )
            .arg(Arg::with_name("SVG_OUTPUT")
                .long("svg")
                .help("Dump the atlas in an SVG file")
                .value_name("SVG_OUTPUT")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("rearrange")
            .about("Rearrange the allocations to reduce fragmentation.")
            .arg(Arg::with_name("ATLAS")
                .short("a")
                .long("atlas")
                .help("Sets the output file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("WIDTH")
                .short("w")
                .long("width")
                .help("Change the width")
                .value_name("WIDTH")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("HEIGHT")
                .short("h")
                .long("height")
                .help("Change the height")
                .value_name("HEIGHT")
                .takes_value(true)
                .required(false)
            )
            .arg(Arg::with_name("SVG_OUTPUT")
                .long("svg")
                .help("Dump the atlas in an SVG file")
                .value_name("SVG_OUTPUT")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("svg")
            .about("Dump the atlas as SVG")
            .arg(Arg::with_name("ATLAS")
                .short("-a")
                .long("atlas")
                .help("Input texture atlas file.")
                .value_name("ATLAS")
                .takes_value(true)
             )
            .arg(Arg::with_name("SVG_OUTPUT")
                .help("Output SVG file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("print")
            .about("Print some stats to stdout")
            .arg(Arg::with_name("ATLAS")
                .short("-a")
                .long("atlas")
                .help("Input texture atlas file.")
                .value_name("ATLAS")
                .takes_value(true)
             )
            .arg(Arg::with_name("SVG_OUTPUT")
                .help("Output SVG file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            SubCommand::with_name("list")
            .about("List the allocations and free rectangles in the atlas")
            .arg(Arg::with_name("ATLAS")
                .short("-a")
                .long("atlas")
                .help("Input texture atlas file.")
                .value_name("ATLAS")
                .takes_value(true)
             )
        )
        .get_matches();

    if let Some(cmd) = matches.subcommand_matches("init") {
        init(&cmd);
    } else if let Some(cmd) = matches.subcommand_matches("allocate") {
        allocate(&cmd);
    } else if let Some(cmd) = matches.subcommand_matches("deallocate") {
        deallocate(&cmd);
    } else if let Some(_cmd) = matches.subcommand_matches("rearrange") {
        //rearrange(&cmd);
    } else if let Some(_cmd) = matches.subcommand_matches("grow") {
        //grow(&cmd);
    } else if let Some(cmd) = matches.subcommand_matches("svg") {
        svg(&cmd);
    } else if let Some(cmd) = matches.subcommand_matches("print") {
        print(&cmd);
    } else if let Some(_cmd) = matches.subcommand_matches("list") {
        //list(&cmd);
    }
}

fn read_atlas(args: &ArgMatches) -> Session {
    let atlas_file_name = args.value_of("ATLAS").unwrap_or("stats.ron");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(atlas_file_name)
        .expect("Failed to open the atlas file.");

    ron::de::from_reader(file).expect("Failed to parse the atlas")
}

fn write_atlas(session: &Session, args: &ArgMatches) {
    let serialized: String =
        ron::ser::to_string_pretty(&session, ron::ser::PrettyConfig::default()).unwrap();

    let atlas_file_name = args.value_of("ATLAS").unwrap_or("stats.ron");
    let mut file =
        std::fs::File::create(atlas_file_name).expect("Failed to open the atlas file.");

    file.write_all(serialized.as_bytes())
        .expect("Failed to write into the atlas file.");
}

fn init(args: &ArgMatches) {

    let session = Session {
        stats: StatsRecorder::new(),
    };

    write_atlas(&session, &args);

    if args.is_present("SVG_OUTPUT") {
        svg(args);
    }
}

fn allocate(args: &ArgMatches) {
    let mut session = read_atlas(args);

    let w = args
        .value_of("WIDTH")
        .expect("Missing width.")
        .parse::<i32>()
        .unwrap();
    let h = args
        .value_of("HEIGHT")
        .expect("Missing height.")
        .parse::<i32>()
        .unwrap();

    session.stats.allocate(size2(w, h));

    write_atlas(&session, args);

    if args.is_present("SVG_OUTPUT") {
        svg(args);
    }
}

fn print(args: &ArgMatches) {
    let session = read_atlas(args);

    session.stats.print();
}

fn deallocate(_args: &ArgMatches) {}

fn svg(_args: &ArgMatches) {}
