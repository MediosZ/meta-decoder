use clap::Parser;
use meta_decoder::process;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let rlib_path = args.path;
    if rlib_path.extension().expect("Unable to get extension.") == "rlib" {
        println!("processing rlib: {:?}", rlib_path);
        process(&rlib_path);
    } else {
        eprintln!("can not recognize the file");
    }
}
