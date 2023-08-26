#![feature(absolute_path)]
/* ----- CDDL HEADER -----
 *
 * The contents of this file are subject to the terms of the
 * Common Development and Distribution License, Version 1.1 only
 * (the "License").  You may not use this file except in compliance
 * with the License.
 *
 * See the file LICENSE in this distribution for details.
 * A copy of the CDDL is also available via the Internet at
 * https://spdx.org/licenses/CDDL-1.1.html
 *
 * When distributing Covered Code, include this CDDL HEADER in each
 * file and include the contents of the LICENSE file from this
 * distribution.
 *
 * ----- CDDL HEADER END -----
 */

use anyhow::Result;
use pest::Parser;
use pest_derive::Parser;
use rfd::FileDialog;
use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Write},
    path::{self, Path, PathBuf},
    str::FromStr,
};

#[derive(Parser)]
#[grammar = "wordstar.pest"]
pub struct WSParser;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let exe_name: Option<String> = env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from);

    // Common usage: ws2markdown <input file> <output file>.
    let inputfile: Option<PathBuf>;
    let mut outputfile: Option<PathBuf> = Some("".into());
    let mut output_to_stdout = false;

    // Output options
    let mut left_margin: usize = 0;

    if &args[1] == "--help" || &args[1] == "-h" {
        // Print usage information.
        println!("ws2markdown: a WordStar to Markdown converter.");
        println!(
            "\tUsage: {0} inputfile.ws [outputfile.md]",
            exe_name.as_deref().unwrap_or("ws2markdown")
        );
        println!("\tIf outputfile = empty, the output will be printed to stdout.");
        return Ok(());
    }
    if args.len() < 3 {
        // Input or output are missing.
        if args.len() < 2 {
            // Ask for the input file.
            inputfile = FileDialog::new()
                .add_filter("WordStar File", &["ws", "ws5", "ws6", "ws7"])
                .set_directory("/")
                .pick_file();
        } else {
            // Input is there, output is missing.
            inputfile = Some(fs::canonicalize(&args[1])?);
        }

        // If no output file was specified: output to stdout.
        output_to_stdout = true;
    } else {
        // Both input and output are there.
        inputfile = Some(fs::canonicalize(&args[1])?);

        // We cannot assume the outputfile to be there yet.
        // -> Don't use canonicalize.
        outputfile = Some(path::absolute(&args[2])?);
    }

    // Read the input file into a string and pass it to the parser.
    // Note that we'll need to disable safe UTF-8 parsing here, because it might
    // well be that WordStar files contain "invalid" UTF-8.
    let mut file_content = Vec::new();
    let mut file = File::open(inputfile.unwrap()).expect("Unable to open file");
    file.read_to_end(&mut file_content).expect("Unable to read");

    let file_content_string = String::from_utf8_lossy(&file_content);

    // The first 128 characters are reserved for the file header.
    let parser = WSParser::parse(Rule::file, &file_content_string[128..])
        .expect("invalid WordStar file!")
        .next()
        .unwrap();

    // Output:
    let mut output_string: String = String::from("");
    for record in parser.into_inner() {
        // DEBUG:
        // println!("{:#?}", record);
        match record.as_rule() {
            Rule::header_line => {
                // h1 to h5
                let headline = &mut record.into_inner();

                // headline[0] -> inner -> rule = dot_h1 .. dot_h5
                let headline_define = headline.next().unwrap().into_inner().peek().unwrap();
                match headline_define.as_rule() {
                    Rule::dot_h1 => output_string.push_str("# "),
                    Rule::dot_h2 => output_string.push_str("## "),
                    Rule::dot_h3 => output_string.push_str("### "),
                    Rule::dot_h4 => output_string.push_str("#### "),
                    Rule::dot_h5 => output_string.push_str("##### "),
                    _ => {}
                }
                // headline[1] -> span -> str = text
                let headline_text = headline.next().unwrap();
                output_string.push_str(headline_text.as_str());

                output_string.push('\n');
            }
            Rule::normal_line => {
                // Add left margin where applicable.
                output_string.push_str(&"&nbsp;".repeat(left_margin));

                // Traverse through the inner pairs.
                let line_pairs = &mut record.into_inner();
                for pair in line_pairs {
                    match pair.as_rule() {
                        // Possible rules:
                        // - displayed_text: just push it
                        // - allowed_modifiers: format first
                        // - everything else: skip
                        Rule::displayed_text => output_string.push_str(pair.as_str()),
                        Rule::allowed_modifiers => {
                            let modifier_pairs = &mut pair.into_inner();
                            for modifier_pair in modifier_pairs {
                                match modifier_pair.as_rule() {
                                    // Possible rules:
                                    // - bold_modifier
                                    // - italics_modifier
                                    // - underline_modifier
                                    Rule::bold_modifier => output_string.push_str("**"),
                                    Rule::italics_modifier => output_string.push('*'),
                                    Rule::underline_modifier => output_string.push_str("__"),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

                output_string.push('\n');
            }
            Rule::dot_command_line => {
                // Right now, these are either one of the allowed_dot_commands
                // or can safely be discarded. There can be only one inner
                // allowed_dot_command per line due to how they are structured.
                let dot_pair = &mut record.into_inner().next().unwrap();
                if dot_pair.as_rule() == Rule::allowed_dot_commands {
                    let dot_command = dot_pair.clone().into_inner().next().unwrap();
                    match dot_command.as_rule() {
                        // Currently possible: dot_insert_file, dot_left_margin, dot_page_break
                        Rule::dot_insert_file => {
                            // This requires a file name.
                            let insert_file_command = dot_command.into_inner().next();
                            if let Some(value) = insert_file_command {
                                // Insert the file as a link.
                                let file_link = format!(
                                    "\n[{0}]({1})\n",
                                    Path::new(value.as_str())
                                        .file_name()
                                        .unwrap()
                                        .to_str()
                                        .unwrap(),
                                    value.as_str()
                                );
                                output_string.push_str(&file_link);
                            }
                        }
                        Rule::dot_left_margin => {
                            // This can either come with a number (set margin) or without
                            // one (reset margin). We shall simulate it with a number of
                            // non-breaking spaces (set left_margin).
                            let left_margin_command = dot_command.into_inner().next();
                            if let Some(value) = left_margin_command {
                                left_margin = usize::from_str(value.as_str()).unwrap_or(0);
                            } else {
                                left_margin = 0;
                            }
                        }
                        Rule::dot_page_break => {
                            // We can't really mirror page breaks in Markdown.
                            // Let's add a horizontal rule instead.
                            output_string.push_str("\n----\n\n");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if output_to_stdout {
        // print the output
        println!("{}", output_string);
    } else {
        // write the output to our output file
        let mut outputfile_handle =
            File::create(outputfile.unwrap()).expect("could not create the output file");
        outputfile_handle
            .write_all(output_string.as_bytes())
            .expect("could not write to the output file");
        println!("Done.");
    }
    Ok(())
}
