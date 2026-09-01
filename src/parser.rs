use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, line_ending, multispace0, space0},
    combinator::{eof, map, opt, value},
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated},
};
use tokio::fs;
use tokio::io;

use crate::types::Repository;

fn skip_comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        preceded(
            char('#'),
            terminated(opt(is_not("\r\n")), alt((line_ending, eof))),
        ),
    )
    .parse(input)
}

fn blank_line(input: &str) -> IResult<&str, ()> {
    value((), terminated(space0, alt((line_ending, eof)))).parse(input)
}

fn clean_inside_block(input: &str) -> IResult<&str, ()> {
    let mut input = input;
    loop {
        if let Ok((rest, _)) = skip_comment(input) {
            input = rest;
            continue;
        }
        if blank_line(input).is_ok() {
            break;
        }
        let (rest, _) = space0(input)?;
        input = rest;
        break;
    }
    Ok((input, ()))
}

fn parse_key_value_line(input: &str) -> IResult<&str, (String, String)> {
    terminated(
        map(
            separated_pair(is_not(":\r\n"), tag(":"), opt(is_not("\r\n"))),
            |(k, v): (&str, Option<&str>)| {
                (k.trim().to_string(), v.unwrap_or("").trim().to_string())
            },
        ),
        alt((line_ending, eof)),
    )
    .parse(input)
}

fn parse_block(input: &str) -> IResult<&str, Repository> {
    let (mut input, _) = clean_inside_block(input)?;
    let mut repo = Repository::new();

    while let Ok((next_input, (key, value))) = parse_key_value_line(input) {
        match key.as_str() {
            "URIs" => {
                repo.uris.extend(value.split_whitespace().map(String::from));
            }
            "Suites" => {
                repo.suites
                    .extend(value.split_whitespace().map(String::from));
            }
            "Components" => {
                repo.components
                    .extend(value.split_whitespace().map(String::from));
            }
            "Signed-By" => {
                repo.signed_by = value;
            }
            _ => (), // Ignore unknown keys
        }

        let (next_input, _) = clean_inside_block(next_input)?;
        input = next_input;
    }

    if !repo.is_valid() {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )))
    } else {
        Ok((input, repo))
    }
}

fn block_separator(input: &str) -> IResult<&str, ()> {
    value((), many1(blank_line)).parse(input)
}

fn parse_file(input: &str) -> IResult<&str, Vec<Repository>> {
    delimited(
        multispace0,
        separated_list1(block_separator, parse_block),
        multispace0,
    )
    .parse(input)
}

pub(crate) async fn read_repositories_from_file(filepath: &str) -> io::Result<Vec<Repository>> {
    let file_content = fs::read_to_string(filepath).await?;

    let (_, repositories) = parse_file(&file_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Parse error: {}", e)))?;
    Ok(repositories)
}
