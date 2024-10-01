#!/usr/bin/env -S cargo +nightly -Zscript

//! Test folder changes

use std::{env, fs::File, ops::Range, path::Path};

enum State {
    Init,
    Add,
    Rm,
}

fn do_range(path: &Path, arg: &str, mut what: impl FnMut(&Path)) {
    let range = parse_range(arg);
    for i in range {
        what(&path.join(format!("cowbump_dummy{}", i)));
    }
}

fn parse_range(input: &str) -> Range<u32> {
    let (first, second) = input.split_once("..").unwrap();
    Range {
        start: first.parse().unwrap(),
        end: second.parse().unwrap(),
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let folder = args.next().expect("Needs directory");
    let path = Path::new(&folder);
    let mut state = State::Init;
    let mut done_anything = false;
    for arg in args {
        match state {
            State::Add => {
                do_range(path, &arg, |path| {
                    if let Err(e) = File::create(path) {
                        eprintln!("File create error: {e}");
                    }
                });
                done_anything = true;
                state = State::Init;
            }
            State::Init => match &arg[..] {
                "add" => state = State::Add,
                "rm" => state = State::Rm,
                _ => panic!("Unknown command '{}'", arg),
            },
            State::Rm => {
                do_range(path, &arg, |path| {
                    if let Err(e) = std::fs::remove_file(path) {
                        eprintln!("File remove error: {e}");
                    }
                });
                done_anything = true;
                state = State::Init;
            }
        }
    }
    if !done_anything {
        eprintln!("Usage: add a..b rm a..b ...");
    }
}
