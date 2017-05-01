#![crate_type= "lib"]
#![cfg_attr(feature = "nightly", feature(plugin))]

#![feature(plugin)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file="clippy.toml")))]

#![cfg_attr(feature = "lints", plugin(clippy))]
#![cfg_attr(feature = "lints", deny(warnings))]
#![cfg_attr(not(any(feature = "lints", feature = "nightly")), deny())]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]

extern crate syntex_syntax;
extern crate syntex_errors;
extern crate itertools;
extern crate walkdir;
extern crate dot;

pub mod prelude;
pub mod syntax;

use std::process::{Command, Stdio};
use std::io::{Write, Read};
use std::path::Path;
use std::fs::{self, File};
use std::ffi::OsStr;
use std::rc::Rc;

use syntex_errors::emitter::ColorConfig;
use syntex_errors::Handler;

use syntex_syntax::codemap::CodeMap;
use syntex_syntax::parse::{self, ParseSess};
use syntex_syntax::{ast, ptr};

use walkdir::WalkDir;
use syntax::ListItem;

/// The default name of *graph/dot* file.
pub const DEFAULT_NAME_DOT: &'static str = "uml-2.5.dot";
/// The default name of *image/png* file.
pub const DEFAULT_NAME_PNG: &'static str = "uml-2.5.png";

/// The function `file2crate` returns a syntex module.
fn file2crate<P: AsRef<Path>>(path: P) -> Option<ast::Crate> {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session: ParseSess = ParseSess::with_span_handler(tty_handler, codemap.clone());
    let parse = parse::parse_crate_from_file(path.as_ref(), &parse_session);

    parse.ok()
}

/// The function `items2chars` returns a graph formated for *Graphiz/Dot*.
fn items2chars(list: Vec<ptr::P<ast::Item>>) -> Option<Vec<u8>> {
    let mut f: Vec<u8> = Vec::new();
    let it: ListItem = ListItem::from(list.iter().peekable());

    dot::render(&it, &mut f).ok().and_then(|()| Some(f))
}

/// The function `rs2dot` returns graphed file module.
pub fn rs2dot<P: AsRef<Path>>(path: P) -> Option<Vec<u8>> {
    file2crate(path).and_then(|parse: ast::Crate| items2chars(parse.module.items))
}

/// The function `src2dot` returns graphed repository of modules.
pub fn src2dot<P: AsRef<Path>>(path: P) -> Option<Vec<u8>> {
    items2chars(WalkDir::new(path).into_iter()
                                 .filter_map(|entry: Result<walkdir::DirEntry, _>| entry.ok())
                                 .filter(|entry| entry.file_type().is_file())
                                 .filter_map(|entry: walkdir::DirEntry| {
                                     let path: &Path = entry.path();

                                     if path.extension().eq(&Some(OsStr::new("rs"))) {
                                         file2crate(path).and_then(|parse| Some(parse.module.items))
                                     } else {
                                         None
                                     }
                                 })
                                 .collect::<Vec<Vec<ptr::P<ast::Item>>>>()
                                 .concat())
}

/// The function `content2png` returns pnged graph content of modules.
fn content2png(buf: Vec<u8>) -> Option<Vec<u8>> {
        Command::new("dot").arg("-Tpng")
                           .stdin(Stdio::piped()).stdout(Stdio::piped())
                           .spawn()
                           .ok()
                           .and_then(|child| {
                                let mut ret = vec![];

                                child.stdin.unwrap().write_all(buf.as_slice()).unwrap();
                                child.stdout.unwrap().read_to_end(&mut ret).unwrap();
                                Some(ret)
                           })
}

/// The function `rs2png` returns pnged file modules.
pub fn rs2png<P: AsRef<Path>>(path: P) -> Option<Vec<u8>> {
    rs2dot(path).and_then(|buf| content2png(buf))
}

/// The function `src2png` returns pnged repository of modules.
pub fn src2png<P: AsRef<Path>>(path: P) -> Option<Vec<u8>> {
    src2dot(path).and_then(|buf| content2png(buf))
}

/// The function `src2both` creates two files formated like a graph/dot and a image/png.
pub fn src2both<P: AsRef<Path>>(src: P, dest: P) -> Option<()> {
    let _ = fs::create_dir_all(dest.as_ref()).unwrap();
    let mut file_dot = File::create(dest.as_ref().join(DEFAULT_NAME_DOT)).unwrap();
    let mut file_png = File::create(dest.as_ref().join(DEFAULT_NAME_PNG)).unwrap();

    src2dot(src).and_then(|content_dot: Vec<u8>| {
        let _ = file_dot.write_all(content_dot.as_slice()).unwrap();

        content2png(content_dot).and_then(|content_png: Vec<u8>| {
            let _ = file_png.write_all(content_png.as_slice()).unwrap();
            Some(())
        })
    })
}
