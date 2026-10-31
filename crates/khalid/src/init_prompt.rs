//! The four questions `khalid init` asks at a terminal (docs/plan.md section 2).
//!
//! Questions go to standard error, so standard output keeps only the result.
//! A question whose flag was given is not asked.

use std::io::{self, BufRead, Write};

use clap::ValueEnum;
use khalid_core::{AbsoluteUrl, Locale};

use crate::cli::{InitArgs, SiteMode};
use crate::init::{self, Answers, mount_path};
use khalid_mount::Stack;

/// Asks for every answer `given` leaves out, reading from `input` and asking
/// on `out`. A wrong answer is explained and the question asked again.
///
/// Fails with [`io::ErrorKind::UnexpectedEof`] when the input ends first.
pub(crate) fn ask(
    given: &InitArgs,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> io::Result<Answers> {
    writeln!(
        out,
        "Khalid asks four questions to set up the blog. Press Enter to take the answer in brackets."
    )?;
    let address = match &given.address {
        Some(address) => address.clone(),
        None => question(
            input,
            out,
            "1 of 4. What is your website's address, such as https://www.example.com?",
            None,
            |text| {
                AbsoluteUrl::parse(text).map_err(|_| {
                    "That is not a full web address. Start it with https:// and the domain."
                        .to_owned()
                })
            },
        )?,
    };
    let mount = match &given.mount {
        Some(mount) => mount_path(mount),
        None => question(
            input,
            out,
            "2 of 4. Which folder of that website should the blog live under, such as /blog/? Use / for the whole website.",
            Some("/"),
            |text| Ok(mount_path(text)),
        )?,
    };
    let languages = if given.languages.is_empty() {
        question(
            input,
            out,
            "3 of 4. Which languages will you write in, such as en or en,de? The first one is the default.",
            Some("en"),
            languages,
        )?
    } else {
        given.languages.clone()
    };
    let mode = match given.mode {
        Some(mode) => mode,
        None => question(
            input,
            out,
            "4 of 4. Should Khalid run the blog as a server, or write plain files you upload (static)?",
            Some("server"),
            |text| SiteMode::from_str(text, true).map_err(|_| "Type server or static.".to_owned()),
        )?,
    };
    let (stack, blog) = stack_and_blog(given, &mount, input, out)?;
    Ok(Answers {
        address,
        mount,
        languages,
        mode,
        stack,
        blog,
    })
}

/// Asks for the stack when the blog lives under a folder of another stack,
/// and where that stack should send it. Both are `None` for the whole site.
fn stack_and_blog(
    given: &InitArgs,
    mount: &str,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> io::Result<(Option<Stack>, Option<AbsoluteUrl>)> {
    if mount == "/" {
        return Ok((None, None));
    }
    let stack = match given.stack {
        Some(stack) => Some(stack),
        None => Some(question(
            input,
            out,
            "The blog lives under a folder. Which stack runs the main site, such as webflow or nginx?",
            None,
            |text| {
                text.parse::<Stack>().map_err(|_| "Type one of: nginx, caddy, apache, traefik, cloudflare-worker, vercel, nextjs, netlify, webflow, shopify.".to_owned())
            },
        )?),
    };
    let blog = match &given.blog {
        Some(blog) => Some(blog.clone()),
        None => Some(question(
            input,
            out,
            "Which address should that stack send the blog to, such as https://blog.example.com?",
            stack
                .filter(|s| init::on_one_machine(*s))
                .map(|_| init::LOCAL_SERVE),
            |text| {
                AbsoluteUrl::parse(text).map_err(|_| {
                    "That is not a full web address. Start it with https:// and the domain."
                        .to_owned()
                })
            },
        )?),
    };
    Ok((stack, blog))
}

/// Asks `prompt` until `parse` accepts the answer. An empty answer takes
/// `default` when there is one.
fn question<T>(
    input: &mut impl BufRead,
    out: &mut impl Write,
    prompt: &str,
    default: Option<&str>,
    parse: impl Fn(&str) -> Result<T, String>,
) -> io::Result<T> {
    loop {
        match default {
            Some(default) => write!(out, "{prompt} [{default}] ")?,
            None => write!(out, "{prompt} ")?,
        }
        out.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            writeln!(out)?;
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let answer = match (line.trim(), default) {
            ("", Some(default)) => default,
            (answer, _) => answer,
        };
        match parse(answer) {
            Ok(value) => return Ok(value),
            Err(why) => writeln!(out, "{why}")?,
        }
    }
}

/// Reads `en,de` as a list of languages.
fn languages(text: &str) -> Result<Vec<Locale>, String> {
    text.split(',')
        .map(|code| {
            let code = code.trim();
            Locale::try_new(code).map_err(|_| {
                format!("{code:?} is not a language code. Use codes such as en, de or pt-BR.")
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use clap::Parser;

    use super::*;
    use crate::cli::{Cli, Command};

    /// The `init` arguments from a command line.
    fn args(line: &[&str]) -> InitArgs {
        let mut full = vec!["khalid", "init"];
        full.extend(line);
        match Cli::parse_from(full).command {
            Some(Command::Init(args)) => args,
            other => panic!("not init: {other:?}"),
        }
    }

    fn run(given: &InitArgs, typed: &str) -> (io::Result<Answers>, String) {
        let mut out = Vec::new();
        let answers = ask(given, &mut Cursor::new(typed), &mut out);
        (answers, String::from_utf8(out).unwrap())
    }

    #[test]
    fn asks_four_questions_and_reads_the_answers() {
        let (answers, asked) = run(
            &args(&[]),
            "https://www.example.com\nblog\nen, de\nStatic\nnginx\nhttps://blog.example.com\n",
        );
        let answers = answers.unwrap();
        assert_eq!(answers.address.as_str(), "https://www.example.com/");
        assert_eq!(answers.mount, "/blog/");
        assert_eq!(answers.languages.len(), 2);
        assert_eq!(answers.mode, SiteMode::Static);
        for number in ["1 of 4", "2 of 4", "3 of 4", "4 of 4"] {
            assert!(asked.contains(number), "{asked}");
        }
    }

    #[test]
    fn enter_sends_a_same_machine_stack_to_khalid_serve() {
        let typed = "https://www.example.com\nblog\n\n\ncaddy\n\n";
        let (answers, asked) = run(&args(&[]), typed);
        let blog = answers.unwrap().blog.unwrap();
        assert_eq!(blog.as_str(), "http://127.0.0.1:8080/");
        assert!(asked.contains("[http://127.0.0.1:8080]"), "{asked}");
    }

    #[test]
    fn enter_takes_the_answer_in_brackets() {
        let (answers, _) = run(&args(&[]), "https://www.example.com\n\n\n\n");
        let answers = answers.unwrap();
        assert_eq!(answers.mount, "/");
        assert_eq!(answers.languages[0].as_str(), "en");
        assert_eq!(answers.mode, SiteMode::Server);
    }

    #[test]
    fn a_wrong_answer_is_explained_and_asked_again() {
        let (answers, asked) = run(
            &args(&[]),
            "example\nhttps://www.example.com\n/\nen,english\nen\nboth\nserver\n",
        );
        assert!(answers.is_ok());
        assert!(asked.contains("not a full web address"), "{asked}");
        assert!(asked.contains("is not a language code"), "{asked}");
        assert!(asked.contains("Type server or static."), "{asked}");
        assert_eq!(asked.matches("1 of 4").count(), 2, "{asked}");
    }

    #[test]
    fn a_flag_skips_its_question() {
        let given = args(&["--address", "https://www.example.com", "--mode", "static"]);
        let (answers, asked) = run(&given, "\n\n");
        assert_eq!(answers.unwrap().mode, SiteMode::Static);
        assert!(!asked.contains("1 of 4"), "{asked}");
        assert!(!asked.contains("4 of 4"), "{asked}");
    }

    #[test]
    fn input_that_ends_early_is_an_error() {
        let (answers, _) = run(&args(&[]), "https://www.example.com\n");
        assert_eq!(answers.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }
}
