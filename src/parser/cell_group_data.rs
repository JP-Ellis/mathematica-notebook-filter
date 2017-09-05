use std::io;

use super::cell::parse_cell_list;
use super::utilities::{load_rest_of_function, check_start};

/// Parse a `CellGroupData`.
///
/// This function requires the input to be at the start of `CellGroupData`.
///
/// Background
/// ==========
///
/// Cells in a Mathematica notebook are not just stored one after the other, but
/// can form a hierarchy.  At its simplest, there is a hierarchy of
/// input--output pairs, but if the Notebook contains various sections and
/// subsections, there can be quite a complex structure.
///
/// The grouping of Cells is done in a `CellGroupData[]` function which has the
/// general form:
///
/// ```mathematica
/// CellGroupData[{
///   cell1,
///   cell2,
///   ...
/// }]
/// ```
///
/// and additionally can have a second argument specifying which cells are open
/// (and all others are closed).
///
/// In addition, optional arguments can be specified after the required
/// arguments.
pub fn parse_cell_group_data<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing CellGroupData.");

    if !check_start(input, b"CellGroupData[")? {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected the start of CellGroupData[].",
        ))
    } else {
        parse_cell_group_data_start(input, output)
            .and(parse_cell_list(input, output))
            .and(parse_cell_group_data_end(input, output))
    }
}

/// Parse and consume the start of the `CellGroupData` up to (but not including)
/// the opening brace of the list of cells, at which point `parse_cell_list`
/// takes over.
fn parse_cell_group_data_start<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing start of CellGroupData.");

    let brace_pos = {
        let buf = input.fill_buf()?;
        buf.iter().position(|&c| c == b'{')
    };

    match brace_pos {
        Some(brace_pos) => {
            {
                let buf = input.fill_buf()?;
                output.write_all(&buf[..brace_pos])?;
            }
            input.consume(brace_pos);
            Ok(())
        }
        None => {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF reached before finding start of the list of cells within CellGroupData[].",
            ))
        }

    }
}

/// Parse the end of the `CellGroupData`.
///
/// This function assumes that the first argument of `CellGroupData` has just
/// been parsed, and that the function is just after the closing brace (either
/// at a comma, or closing bracket if there are no additional arguments to
/// `CellGroupData`).
///
/// Since `CellGroupData` does not appear to store any optional information that
/// ought to be stripped, this function simply consumes everything up to the
/// closing bracket of the function.
fn parse_cell_group_data_end<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing end of CellGroupData.");

    let (s, _) = load_rest_of_function(input)?;
    match s.first() {
        Some(&b',') | Some(&b']') => output.write_all(&s),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected to be right after an argument when parsing the end of CellGroupData[].",
        )),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn cell_group_data_start() {
        let mut output = Vec::new();
        let mut input = &b"CellGroupData[{\nCell["[..];
        assert!(super::parse_cell_group_data_start(&mut input, &mut output).is_ok());
        assert_eq!(input, &b"{\nCell["[..]);
        assert_eq!(output, &b"CellGroupData["[..]);
    }

    #[test]
    fn cell_group_data_end() {
        let mut output = Vec::new();
        let mut input = &b", Open], Cell[{}, \"Output\"]"[..];
        assert!(super::parse_cell_group_data_end(&mut input, &mut output).is_ok());
        assert_eq!(input, &b", Cell[{}, \"Output\"]"[..]);
        assert_eq!(output.as_slice(), &b", Open]"[..]);

        let mut output = Vec::new();
        let mut input = &b"Open], Cell[{}, \"Output\"]"[..];
        assert!(super::parse_cell_group_data_end(&mut input, &mut output).is_err());
    }

    #[test]
    fn cell_group_data() {
        let mut output = Vec::new();
        let mut input = &b"CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]}, Open], Foo"[..];
        assert!(super::parse_cell_group_data(&mut input, &mut output).is_ok());
        assert_eq!(input, &b", Foo"[..]);
        assert_eq!(output, &b"CellGroupData[{Cell[1, \"Input\"]}, Open]"[..]);

        let mut output = Vec::new();
        let mut input = &b"CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]} Open], Foo"[..];
        assert!(super::parse_cell_group_data(&mut input, &mut output).is_err());

        let mut output = Vec::new();
        let mut input = &b"Foo[CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]} Open], Foo"
            [..];
        assert!(super::parse_cell_group_data(&mut input, &mut output).is_err());
    }
}
