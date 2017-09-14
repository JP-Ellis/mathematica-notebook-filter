use std::cmp;
use std::io;

use super::cell_group_data::parse_cell_group_data;
use super::utilities::{load_function, read_consume_output, check_start};

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
    debug!("Parsing cell list.");

    if !check_start(input, b"{")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected the start of a list of cells.",
        ));
    }

    // Note that although we could load the full list, it could actually be
    // *very* long so we parse each cell individually.

    // To make parsing the loop easier later, we'll output the opening brace now
    // and everything up to the first cell, or up to and including the closing
    // brace if there are no cells in the list.  In the latter case, we can exit
    // the function early.
    let cell_bytes = b"Cell[";
    let (next_cell, next_brace) = {
        let buf = input.fill_buf()?;
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
            // Write everything up to and including the next brace
            read_consume_output(input, output, p + 1)?;
            return Ok(());
        }
        (Some(p1), Some(p)) if p < p1 => {
            // Write everything up to and including the next brace
            read_consume_output(input, output, p + 1)?;
            return Ok(());
        }
        (Some(p), None) => {
            // Write everything up to the next Cell
            read_consume_output(input, output, p)?;
        }
        (Some(p), Some(p2)) if p < p2 => {
            // Write everything up to the next Cell
            read_consume_output(input, output, p)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unexpected branch reached.  This is a bug and should be reported.",
            ))
        }
    };

    // At this stage, `input` is at the start of the first `Cell[]`, with the
    // opening brace already having been outputted.  Note that if we happen to
    // have omitted cells at the start of the list, we do not want to output a
    // separating comma, so we must keep track of when we still are at the start
    // of the list.
    let mut cell_buf = Vec::new();
    let mut start_of_list = true;
    loop {
        cell_buf.clear();

        // Get the positions of the next `Cell[` and `}`.
        let (next_cell, next_brace) = {
            let buf = input.fill_buf()?;
            (
                buf.windows(cell_bytes.len()).position(|w| w == cell_bytes),
                buf.iter().position(|&c| c == b'}'),
            )
        };

        // If we have a closing brace coming up first, we've reached the end of
        // the list so we can output up to that and exit.  If instead we have a
        // Cell coming up, we need to parse the Cell and check whether it will
        // need to be outputted.
        match (next_cell, next_brace) {
            (None, None) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unable to locate the next cell or end of list in list of cells.",
                ))
            }
            (None, Some(p)) => {
                // Write everything up to and including the closing brace
                read_consume_output(input, output, p + 1)?;
                return Ok(());
            }
            (Some(p1), Some(p)) if p < p1 => {
                // Write everything up to and including the closing brace
                read_consume_output(input, output, p + 1)?;
                return Ok(());
            }
            (Some(p), None) => {
                // Load everything up to the cell (which will include the
                // preceding comma.
                if start_of_list {
                    input.consume(p);
                } else {
                    read_consume_output(input, &mut cell_buf, p)?;
                }
                if parse_cell(input, &mut cell_buf)? {
                    output.write_all(&cell_buf)?;
                    start_of_list = false;
                }
            }
            (Some(p), Some(p2)) if p < p2 => {
                // Load everything up to the cell (which will include the
                // preceding comma.
                if start_of_list {
                    input.consume(p);
                } else {
                    read_consume_output(input, &mut cell_buf, p)?;
                }
                if parse_cell(input, &mut cell_buf)? {
                    output.write_all(&cell_buf)?;
                    start_of_list = false;
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unexpected branch reached.  This is a bug and should be reported.",
                ))
            }
        }
    }
}

/// Parse a `Cell[]`, and only send it to the output if it is not an output
/// cell.  This function also removes metadata associated with the cell.
///
/// The input must be at the start of the Cell function.
///
/// If anything was outputted by the function, the function will return `true`
/// (and `false` otherwise) so that the parsing of list of cells can omit commas
/// as appropriate.
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
/// and optional arguments can be specified at the end.  We will assume that
/// optional arguments are *only* used in the latter format.
///
/// The omitted styles are:
///
/// - Output
/// - Print
/// - Message
pub fn parse_cell<I, O>(input: &mut I, output: &mut O) -> Result<bool, io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing Cell.");

    if !check_start(input, b"Cell[")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected the start of a Cell[].",
        ));
    }

    // The input is now at the start of the cell.  We load the full function
    // into one vector, and also locate its various arguments.
    let (cell_bytes, args) = load_function(input)?;
    let num_args = args.len() - 1;

    // Check the second argument of `Cell[]` to see whether it is to be deleted
    // in which case we can ignore the Cell completely.
    let to_ignore = vec![
        &b"\"Message\""[..],
        &b"\"Output\""[..],
        &b"\"PrintTemporary\""[..],
        &b"\"Print\""[..],
    ];
    if num_args >= 2 {
        let is_to_ignore = to_ignore.iter().any(|&cell_type| {
            cell_bytes[args[1]..args[2]].windows(cell_type.len()).any(
                |w| {
                    w == cell_type
                },
            )
        });
        if is_to_ignore {
            return Ok(false);
        }
    }

    // At this stage, we know we are outputting the cell.

    // Inspect the start of the first argument and see if it contains
    // `CellGroupData`.  Because of formatting it need not be right at the start
    // of the first argument, but we also want to avoid inspecting too far into
    // the cell in case `CellGroupData[]` was inputted by the user.
    let cell_group_data_bytes = b"CellGroupData[";
    let pos = cell_bytes[args[0]..cmp::min(args[1], args[0] + 2 * cell_group_data_bytes.len())]
        .windows(cell_group_data_bytes.len())
        .position(|w| w == cell_group_data_bytes);

    // When outputting the arguments, we specifically omit the trailing comma.
    match (pos, num_args) {
        (_, 0) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cell[] must have at least one argument.",
            ))
        }
        (None, 1) => output.write_all(&cell_bytes[..args[1] - 1])?,
        (None, _) => output.write_all(&cell_bytes[..args[2] - 1])?,
        (Some(p), 1) => {
            output.write_all(&cell_bytes[..args[0] + p])?;
            parse_cell_group_data(&mut &cell_bytes[args[0] + p..args[1] - 1], output)?;
        }
        (Some(p), _) => {
            output.write_all(&cell_bytes[..args[0] + p])?;
            parse_cell_group_data(&mut &cell_bytes[args[0] + p..args[1] - 1], output)?;
            output.write_all(&cell_bytes[args[1] - 1..args[2] - 1])?;
        }
    };

    // We have now outputted the first two arguments of `Cell`.  It remains to
    // parse the optional arguments.
    let opt_to_exclude = vec![&b"CellChangeTimes"[..], &b"ExpressionUUID"[..]];
    for arg in 2..num_args {
        let is_to_ignore = opt_to_exclude.iter().any(|&opt| {
            cell_bytes[args[arg]..args[arg + 1] - 1]
                .windows(opt.len())
                .any(|w| w == opt)
        });
        if !is_to_ignore {
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
        // Output Cells
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
        assert_eq!(&output, br#"Cell[123, "Input"]"#);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", Foo->Bar]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(&output, br#"Cell[123, "Input", Foo->Bar]"#);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", CellChangeTimes->123]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(&output, br#"Cell[123, "Input"]"#);

        let mut output = Vec::new();
        let mut input = &br#"Cell[123, "Input", CellChangeTimes->123, Foo->Bar]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(&output, br#"Cell[123, "Input", Foo->Bar]"#);

        // Cell with CellGroupData
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input =
            &b"Cell[CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]}, Open]], Foo"[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert_eq!(input, b", Foo");
        assert_eq!(output, &br#"Cell[CellGroupData[{Cell[1, "Input"]}]]"#[..]);

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
        assert_eq!(
            output,
            &br#"Cell[
CellGroupData[{
  Cell[CellGroupData[{Cell[1, "Input"]}]],
  Cell[3, "Input"]
}]]"#
                [..]
        );

        // Invalid Cells
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br#"Foo Cell[123, "Input"]"#[..];
        assert!(super::parse_cell(&mut input, &mut output).is_err());

        // Although technically invalid from the Mathematica notebook, this is
        // outputted fine.
        let mut output = Vec::new();
        let mut input = &b"Cell[]"[..];
        assert!(super::parse_cell(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, b"Cell[]");
    }

    #[test]
    fn cell_list() {
        // Simple lists
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"{} Foo"[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert_eq!(input, b" Foo");
        assert_eq!(output, b"{}");

        let mut output = Vec::new();
        let mut input = &b"{Cell[1]} Cell[2]"[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert_eq!(input, b" Cell[2]");
        assert_eq!(output, b"{Cell[1]}");

        // Combinations of Input and Output
        ////////////////////////////////////////
        // Valid list of 3 cells with anything from 0 to 3 output cells, in all
        // possible configurations.
        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[1], Cell[2], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[2], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2, "Output"], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[1], Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[1], Cell[2]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1], Cell[2, "Output"], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[1]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[2]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2, "Output"], Cell[3]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{Cell[3]}"#[..]);

        let mut output = Vec::new();
        let mut input = &br#"{Cell[1, "Output"], Cell[2, "Output"], Cell[3, "Output"]}"#[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_ok());
        assert!(input.is_empty());
        assert_eq!(output, &br#"{}"#[..]);

        // Invalid Cell lists
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"Foo"[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_err());

        let mut output = Vec::new();
        let mut input = &b"{Foo"[..];
        assert!(super::parse_cell_list(&mut input, &mut output).is_err());
    }
}
