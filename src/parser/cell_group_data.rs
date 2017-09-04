use std::io;

use super::cell::parse_cell_list;
use super::utilities::load_rest_of_function;

/// Parse a `CellGroupData`.
///
/// This function will advance through the input until `CellGroupData` is
/// reached.
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
    parse_cell_group_data_start(input, output)
        .and(parse_cell_list(input, output))
        .and(parse_cell_group_data_end(input, output))
}

/// Parse the start of the `CellGroupData`
fn parse_cell_group_data_start<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    let cell_group_data_bytes = &b"CellGroupData["[..];

    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(cell_group_data_bytes.len()).position(|w| {
            w == cell_group_data_bytes
        })
    };

    match pos {
        Some(pos) => {
            input.consume(pos);

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
                        "EOF reached before finding start of the list of cells within `CellGroupData[]`.",
                    ))
                }

            }
        }
        None => {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF reached before finding the `CellGroupData[]` function.",
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
fn parse_cell_group_data_end<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    // It appears that there is nothing to remove from the `GroupDataCell`, so
    // we just parse the rest of the function and output it.
    let (s, _) = load_rest_of_function(input)?;
    match s.first() {
        Some(&b',') | Some(&b']') => output.write_all(&s),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected to be right after an argument when parsing the end of `CellGroupData[]`.",
        )),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn cell_group_data_start() {
        let mut output = Vec::new();
        let mut input = &b"Cell[\nCellGroupData[{\nCell["[..];
        assert!(super::parse_cell_group_data_start(&mut input, &mut output).is_ok());
        assert_eq!(input, &b"{\nCell["[..]);
        assert_eq!(output.as_slice(), &b"CellGroupData["[..]);
    }

    #[test]
    fn cell_group_data_end() {
        let mut output = Vec::new();
        let mut input = &b", Open], Cell[{}, \"Output\"]"[..];
        assert!(super::parse_cell_group_data_end(&mut input, &mut output).is_ok());
        assert_eq!(input, &b", Cell[{}, \"Output\"]"[..]);
        assert_eq!(output.as_slice(), &b", Open]"[..]);
    }

    #[test]
    fn cell_group_data() {
        let mut output = Vec::new();
        let mut input = &b"CellGroupData[{Cell[1, \"Input\"], Cell[2, \"Output\"]}, Open], Foo"[..];
        assert!(super::parse_cell_group_data(&mut input, &mut output).is_ok());
        assert_eq!(input, &b", Foo"[..]);
        assert_eq!(
            output.as_slice(),
            &b"CellGroupData[{Cell[1, \"Input\"]}, Open]"[..]
        );
    }
}
