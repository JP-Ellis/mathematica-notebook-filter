use std::cmp;
use std::io;
use std::io::Write;

use super::cell_group_data::parse_cell_group_data;
use super::utilities::load_function;

/// Parse a list of `Cell[]` of the form:
///
/// ```mathematica
/// {Cell[...], Cell[...], ...}
/// ```
///
/// This function requires the input to be at the opening brace and will consume
/// everything up to and including the closing brace of the list.
///
/// If a `Cell` is omitted (due to being an output `Cell`), this function will
/// ensure that the separating commas are appropriately adjusted as the Wolfram
/// language does not allow dangling commas.
pub fn parse_cell_list<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    let cell_bytes = &b"Cell["[..];

    // Note that although we could load the full list, it could actually be
    // *very* long.  Instead, we'll keep parsing the list as we go.

    // We write the opening brace of the list.  We just double check that the
    // first character is an opening brace.  In order to keep some of the
    // formatting, we'll also write up to the start of the first `Cell[]` (or to
    // the end of the list if there are no cells).
    let (next_cell, next_brace) = {
        let buf = input.fill_buf()?;
        match buf.first() {
            Some(&b'{') => (),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected the start of a list of cells.",
                ))
            }
        }
        (
            buf.windows(cell_bytes.len()).position(|w| w == cell_bytes),
            buf.iter().position(|&c| c == b'}'),
        )
    };

    match (next_cell, next_brace) {
        (None, None) => {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unable to locate the next cell or end of list in list of cells.",
            ))
        }
        (None, Some(p)) => {
            // End of list is coming up first (no cell in sight)
            {
                let buf = input.fill_buf()?;
                output.write_all(&buf[..p + 1])?;
            }
            input.consume(p + 1);
            return Ok(());
        }
        (Some(p), None) => {
            // Cell is coming up
            {
                let buf = input.fill_buf()?;
                output.write_all(&buf[..p])?;
            }
            input.consume(p);
        }
        (Some(p), Some(p2)) if p < p2 => {
            // Cell is coming up
            {
                let buf = input.fill_buf()?;
                output.write_all(&buf[..p])?;
            }
            input.consume(p);
        }
        (Some(p1), Some(p)) if p < p1 => {
            // End of list is coming
            {
                let buf = input.fill_buf()?;
                output.write_all(&buf[..p + 1])?;
            }
            input.consume(p + 1);
            return Ok(());
        }
        _ => panic!("Unexpected branch encountered."),
    };

    // At this stage, `input` should be at the start of the first `Cell[]`.

    // We'll be keeping track of the previous separated (if any) since we don't
    // know whether it should be outputted until the Cell has been parsed.  We
    // also have to handle the start of the list (since we *do not* want to add
    // an initial comma).
    let mut start_of_list = true;
    let mut prev_separator = Vec::new();
    loop {
        // Get the positions of the next `Cell[` and closing brace.
        let (next_cell, next_brace) = {
            let buf = input.fill_buf()?;
            (
                buf.windows(cell_bytes.len()).position(|w| w == cell_bytes),
                buf.iter().position(|&c| c == b'}'),
            )
        };

        // Check whether we have a Cell first, or the end of the list and act as
        // appropriate.  Since we don't know whether a Cell will generate any
        // output until it has been parsed, we can't write the separator as we
        // go (or we might double up on commas).
        //
        // Additionally, there's the special case of an empty list to handle,
        // hence we keep track of the `prev_separator` which, at the start of
        // the list, contains the opening brace.
        match (next_cell, next_brace) {
            (None, None) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unable to locate the next cell or end of list in list of cells.",
                ))
            }
            (None, Some(p)) => {
                // End of list is coming up first (no cell in sight)
                {
                    let buf = input.fill_buf()?;
                    output.write_all(&buf[..p + 1])?;
                }
                input.consume(p + 1);
                return Ok(());
            }
            (Some(p), None) => {
                // Cell is coming up
                {
                    let buf = input.fill_buf()?;
                    prev_separator.write_all(&buf[..p])?;
                }
                input.consume(p);

                let mut cell_buf = Vec::new();
                match (parse_cell(input, &mut cell_buf)?, start_of_list) {
                    (false, _) => {}
                    (true, true) => {
                        output.write_all(&cell_buf)?;
                        start_of_list = false;
                    }
                    (true, false) => {
                        output.write_all(&prev_separator)?;
                        output.write_all(&cell_buf)?;
                        start_of_list = false;
                    }
                }
            }
            (Some(p), Some(p2)) if p < p2 => {
                // Cell is coming up
                {
                    let buf = input.fill_buf()?;
                    prev_separator.write_all(&buf[..p])?;
                }
                input.consume(p);

                let mut cell_buf = Vec::new();
                match (parse_cell(input, &mut cell_buf)?, start_of_list) {
                    (false, _) => {}
                    (true, true) => {
                        output.write_all(&cell_buf)?;
                        start_of_list = false;
                    }
                    (true, false) => {
                        output.write_all(&prev_separator)?;
                        output.write_all(&cell_buf)?;
                        start_of_list = false;
                    }
                }
            }
            (Some(p1), Some(p)) if p < p1 => {
                // End of list is coming
                {
                    let buf = input.fill_buf()?;
                    output.write_all(&buf[..p + 1])?;
                }
                input.consume(p + 1);
                return Ok(());
            }
            _ => panic!("Unexpected branch encountered."),
        }

        prev_separator.clear();
    }
}

/// Parse a `Cell[]`, and only send it to the output if it is not an output
/// cell.  This function also removes metadata associated with the cell.
///
/// The input will be consumed until the first encountered Cell.
///
/// If any of the input was passed on to the output, the `Ok` value `true`.  If
/// not, the `Ok` value will be `false` and it is up to the calling function to
/// correctly handle the addition or omission separating commas since the
/// Wolfram language does not allow dangling commas.
///
/// Background
/// ==========
///
/// The `Cell` functions takes one of two forms:
///
/// ```mathematica
/// Cell[contents]
/// ```
///
/// and
///
/// ```mathematica
/// Cell[contents, "style"]
/// ```
///
/// and optional arguments can be specified at the end.
pub fn parse_cell<I, O>(input: &mut I, output: &mut O) -> Result<bool, io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    let cell_bytes = &b"Cell["[..];

    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(cell_bytes.len()).position(|w| w == cell_bytes)
    };

    match pos {
        Some(pos) => {
            input.consume(pos);
        }
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to locate the beginning of Notebook content specification.",
            ));
        }
    }

    // The input is now at the start of the cell.  We load the full function
    // into one vector, and also locate its various arguments.
    let (cell_bytes, args) = load_function(input)?;
    let num_args = args.len() - 1;

    // Check the second argument of `Cell[]` to see whether it is to be deleted
    // in which case we can ignore the Cell completely.
    let to_ignore = vec![
        &br#""Output""#[..],
        &br#""Print""#[..],
        &br#""Message""#[..],
    ];
    if num_args >= 2 &&
        to_ignore.iter().any(|&cell_type| {
            cell_bytes[args[1]..args[2]].windows(cell_type.len()).any(
                |w| {
                    w == cell_type
                },
            )
        })
    {
        return Ok(false);
    }

    // At this stage, we know we are outputting the cell.  We can write the
    // function call itself.
    output.write_all(&cell_bytes[..args[0]])?;

    // Inspect the start of the first argument and see if it contains
    // `CellGroupData`.  Note we don't want to go too far into the `Cell` as it
    // has some user content.
    let cell_group_data_bytes = &b"CellGroupData["[..];
    let pos = cell_bytes[args[0]..cmp::min(args[1], args[0] + 2 * cell_group_data_bytes.len())]
        .windows(cell_group_data_bytes.len())
        .position(|w| w == cell_group_data_bytes);

    // When outputting the arguments, we specifically omit the closing one.  We
    // need to add it in later *if* there are arguments that follow.
    match (pos, num_args) {
        (_, 0) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cell[] must have at least one argument.",
            ))
        }
        (None, 1) => output.write_all(&cell_bytes[args[0]..args[1] - 1])?,
        (None, _) => output.write_all(&cell_bytes[args[0]..args[2] - 1])?,
        (Some(p), 1) => {
            output.write_all(&cell_bytes[args[0]..args[0] + p])?;
            // Need to double check whether we can pass a reference without it
            // being modified, or whether it will cause an error with indexing later.
            parse_cell_group_data(&mut &cell_bytes[args[0] + p..args[1] - 1], output)?;
        }
        (Some(p), _) => {
            output.write_all(&cell_bytes[args[0]..args[0] + p])?;
            // Need to double check whether we can pass a reference without it
            // being modified, or whether it will cause an error with indexing later.
            parse_cell_group_data(&mut &cell_bytes[args[0] + p..args[1] - 1], output)?;
            output.write_all(&cell_bytes[args[1] - 1..args[2] - 1])?;
        }
    };

    // We have now outputed the first two arguments of `Cell`.  Options which we
    // don't want will be excluded.
    let to_exclude = vec![&b"CellChangeTimes"[..], &b"ExpressionUUID"[..]];
    for arg in 2..num_args {
        if !to_exclude.iter().any(|&opt| {
            cell_bytes[args[arg]..args[arg + 1] - 1]
                .windows(opt.len())
                .any(|w| w == opt)
        })
        {
            output.write_all(
                &cell_bytes[args[arg] - 1..args[arg + 1] - 1],
            )?;
        }
    }

    // Since we purposefully exclude the trailing comma/bracket, we write it out now.
    output.write_all(b"]")?;

    Ok(true)
}

#[cfg(test)]
mod test {
    #[test]
    fn cell() {
        // Output cells
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Output"]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert!(output.is_empty());

        let mut output = Vec::new();
        let mut input = &br#"Cell[{Cell[1, "Input"], Cell[2, "Output"]}, "Output"]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert!(output.is_empty());

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Output", MetaData->Foo, Timestamp->123]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert!(output.is_empty());

        // Non-output Cells
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input"]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"Cell[123, "Input"]"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", Foo->Bar]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"Cell[123, "Input", Foo->Bar]"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", CellChangeTimes->123]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"Cell[123, "Input"]"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", CellChangeTimes->123, Foo->Bar]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"Cell[123, "Input", Foo->Bar]"#[..]);

        // Cell with CellGroupData
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input =
            &b"Cell[CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]}, Open]], Foo"[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert_eq!(input, &b", Foo"[..]);
        assert_eq!(
            output.as_slice(),
            &b"Cell[CellGroupData[{Cell[1, \"Input\"]}, Open]]"[..]
        );

        let mut output = Vec::new();
        let mut input = &br#"Cell[
CellGroupData[{
  Cell[CellGroupData[{Cell[1, "Input"], Cell[2, "Output"]}]],
  Cell[3, "Input"],
  Cell[4, "Output"]
}, Open]],
Foo"#
            [..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert_eq!(input, &b",\nFoo"[..]);
        println!("Output: {}", String::from_utf8(output.clone()).unwrap());
        assert_eq!(
            output.as_slice(),
            &br#"Cell[
CellGroupData[{
  Cell[CellGroupData[{Cell[1, "Input"]}]],
  Cell[3, "Input"]
}, Open]]"#
                [..]
        );
    }

    #[test]
    fn cell_list() {
        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[1], Cell[2], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[2], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2, "Output"], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[1], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[1], Cell[2]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2, "Output"], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[1]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[2]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2, "Output"], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2, "Output"], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output.as_slice(), &br#"{}"#[..]);
    }
}
